pub mod managed;

pub use managed::manage;
use std::env;

use anyhow::Context as _;
use raphy_protocol::{ClientToServerMessage, Config, Operation, ServerToClientMessage, TaskId};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpStream, ToSocketAddrs, UnixStream, tcp, unix};

enum OwnedReadHalf {
    Tcp(tcp::OwnedReadHalf),
    Unix(unix::OwnedReadHalf),
}

impl AsyncRead for OwnedReadHalf {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
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
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
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

#[derive(Error, Debug)]
pub enum RecvMessageError {
    #[error("i/o error")]
    Io(#[from] io::Error),

    #[error("bincode decode error")]
    Bincode(#[from] bincode::error::DecodeError),
}

pub struct ClientReader(OwnedReadHalf);

impl ClientReader {
    pub async fn recv(&mut self) -> Result<ServerToClientMessage, RecvMessageError> {
        let mut len = [0; 4];
        self.0.read_exact(&mut len).await?;

        let mut buf = vec![0; u32::from_le_bytes(len) as usize];
        self.0.read_exact(&mut buf).await?;

        bincode::decode_from_slice(&buf, bincode::config::standard())
            .map(|(m, _)| m)
            .map_err(Into::into)
    }

    pub fn is_unix(&self) -> bool {
        matches!(&self.0, OwnedReadHalf::Unix(_))
    }

    pub fn is_tcp(&self) -> bool {
        matches!(&self.0, OwnedReadHalf::Tcp(_))
    }
}

#[derive(Error, Debug)]
pub enum SendMessageError {
    #[error("i/o error")]
    Io(#[from] io::Error),

    #[error("bincode encode error")]
    Bincode(#[from] bincode::error::EncodeError),
}

pub struct ClientWriter(OwnedWriteHalf);

impl ClientWriter {
    async fn send_message(
        &mut self,
        message: ClientToServerMessage,
    ) -> Result<(), SendMessageError> {
        let data = bincode::encode_to_vec(message, bincode::config::standard())?;
        let mut buf = Vec::with_capacity(4 + data.len());
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
        buf.extend(data);
        tracing::debug!(?buf);
        self.0.write_all(&buf).await.map_err(Into::into)
    }
    
    pub async fn ping(&mut self) -> Result<TaskId, SendMessageError> {
        let task_id = TaskId::generate();
        self.send_message(ClientToServerMessage::Ping(task_id))
            .await?;
        Ok(task_id)
    }

    pub async fn get_config(&mut self) -> Result<TaskId, SendMessageError> {
        let task_id = TaskId::generate();
        self.send_message(ClientToServerMessage::GetConfig(task_id))
            .await?;
        Ok(task_id)
    }

    pub async fn update_config(&mut self, config: Config) -> Result<TaskId, SendMessageError> {
        let task_id = TaskId::generate();
        self.send_message(ClientToServerMessage::UpdateConfig(task_id, config))
            .await?;
        Ok(task_id)
    }

    pub async fn perform_operation(
        &mut self,
        operation: Operation,
    ) -> Result<TaskId, SendMessageError> {
        let task_id = TaskId::generate();
        self.send_message(ClientToServerMessage::PerformOperation(task_id, operation))
            .await?;
        Ok(task_id)
    }

    pub async fn input(&mut self, input: Vec<u8>) -> Result<(), SendMessageError> {
        self.send_message(ClientToServerMessage::Input(input)).await
    }

    pub async fn shutdown(&mut self) -> Result<(), SendMessageError> {
        self.send_message(ClientToServerMessage::Shutdown).await
    }
}

impl ClientWriter {
    pub fn is_unix(&self) -> bool {
        matches!(&self.0, OwnedWriteHalf::Unix(_))
    }

    pub fn is_tcp(&self) -> bool {
        matches!(&self.0, OwnedWriteHalf::Tcp(_))
    }
}

pub async fn from_tcp(addrs: impl ToSocketAddrs) -> io::Result<(ClientReader, ClientWriter)> {
    tracing::debug!("tcp stream connect");
    let stream = TcpStream::connect(addrs).await?;
    tracing::debug!("tcp stream connected");

    let (read_half, write_half) = stream.into_split();

    Ok((
        ClientReader(OwnedReadHalf::Tcp(read_half)),
        ClientWriter(OwnedWriteHalf::Tcp(write_half)),
    ))
}

pub async fn from_unix(addr: impl AsRef<Path>) -> io::Result<(ClientReader, ClientWriter)> {
    tracing::debug!("unix stream connect");
    let stream = UnixStream::connect(addr).await?;
    tracing::debug!("unix stream connected");

    let (read_half, write_half) = stream.into_split();

    Ok((
        ClientReader(OwnedReadHalf::Unix(read_half)),
        ClientWriter(OwnedWriteHalf::Unix(write_half)),
    ))
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum ClientMode {
    Local,
    Remote,
}

impl ClientMode {
    pub fn get() -> anyhow::Result<Self> {
        let current_exe =
            env::current_exe().context("Failed to determine the current executable path.")?;
        let current_exe = current_exe
            .file_name()
            .context("The current executable path has no file name.")?
            .to_str();

        match current_exe {
            Some("raphy-local-client-app") => Ok(Self::Local),
            Some("raphy-remote-client-app") => Ok(Self::Remote),
            other => anyhow::bail!(
                "The client mode could not be determined from the current executable path {other:?}."
            ),
        }
    }
}
