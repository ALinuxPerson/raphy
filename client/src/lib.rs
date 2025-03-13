use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{tcp, unix, TcpStream, ToSocketAddrs, UnixStream};
use raphy_protocol::{Config, Operation};

enum OwnedReadHalf {
    Tcp(tcp::OwnedReadHalf),
    Unix(unix::OwnedReadHalf),
}

impl AsyncRead for OwnedReadHalf {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(half) => Pin::new(half).poll_read(cx, buf),
            Self::Unix(half) => Pin::new(half).poll_read(cx, buf),
        }
    }
}

enum OwnedWriteHalf {
    Tcp(tcp::OwnedWriteHalf),
    Unix(unix::OwnedWriteHalf),
}

impl AsyncWrite for OwnedWriteHalf {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Tcp(half) => Pin::new(half).poll_write(cx, buf),
            Self::Unix(half) => Pin::new(half).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(half) => Pin::new(half).poll_flush(cx),
            Self::Unix(half) => Pin::new(half).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(half) => Pin::new(half).poll_shutdown(cx),
            Self::Unix(half) => Pin::new(half).poll_shutdown(cx),
        }
    }
}

pub struct ClientReader(OwnedReadHalf);

pub struct ClientWriter(OwnedWriteHalf);

impl ClientWriter {
    pub async fn update_config(&self, config: Config) -> io::Result<()> {
        todo!()
    }
    
    pub async fn perform_operation(&self, operation: Operation) -> io::Result<()> {
        todo!()
    }
}

pub async fn from_tcp(addrs: impl ToSocketAddrs) -> io::Result<(ClientReader, ClientWriter)> {
    let stream = TcpStream::connect(addrs).await?;
    let (read_half, write_half) = stream.into_split();

    Ok((ClientReader(OwnedReadHalf::Tcp(read_half)), ClientWriter(OwnedWriteHalf::Tcp(write_half))))
}

pub async fn from_unix(addr: impl AsRef<Path>) -> io::Result<(ClientReader, ClientWriter)> {
    let stream = UnixStream::connect(addr).await?;
    let (read_half, write_half) = stream.into_split();

    Ok((ClientReader(OwnedReadHalf::Unix(read_half)), ClientWriter(OwnedWriteHalf::Unix(write_half))))
}