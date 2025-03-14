pub mod managed {
    use anyhow::Context;
    use raphy_protocol::{Config, Operation, ServerToClientMessage};
    use std::io;
    use std::path::Path;
    use thiserror::Error;
    use tokio::net::ToSocketAddrs;
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
    use tokio::sync::{broadcast, mpsc, oneshot};
    use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle};

    pub struct ClientReader(broadcast::Receiver<ServerToClientMessage>);

    impl ClientReader {
        pub async fn recv(&mut self) -> Option<ServerToClientMessage> {
            self.0.recv().await.ok()
        }

        pub async fn expect(
            &mut self,
            mut f: impl FnMut(&ServerToClientMessage) -> bool,
        ) -> Option<ServerToClientMessage> {
            loop {
                let message = self.recv().await?;

                if f(&message) {
                    return Some(message);
                }
            }
        }
    }

    impl Clone for ClientReader {
        fn clone(&self) -> Self {
            Self(self.0.resubscribe())
        }
    }

    #[derive(Debug, Error)]
    #[error("not a local client")]
    pub struct NotALocalClient;

    enum ClientToServerMessage {
        UpdateConfig(Config, oneshot::Sender<()>),
        PerformOperation(Operation, oneshot::Sender<anyhow::Result<()>>),
        Input(Vec<u8>),
        Shutdown(oneshot::Sender<Result<(), NotALocalClient>>),
    }

    #[derive(Clone)]
    pub struct ClientWriter(UnboundedSender<ClientToServerMessage>);

    impl ClientWriter {
        pub async fn update_config(&self, config: Config) -> anyhow::Result<()> {
            let (tx, rx) = oneshot::channel();
            self.0
                .send(ClientToServerMessage::UpdateConfig(config, tx))
                .context("failed to send update config message")?;
            rx.await.context("failed to update config")
        }

        pub async fn perform_operation(&self, operation: Operation) -> anyhow::Result<()> {
            let (tx, rx) = oneshot::channel();
            self.0
                .send(ClientToServerMessage::PerformOperation(operation, tx))
                .context("failed to send perform operation message")?;
            rx.await
                .context("tx dropped")?
                .context("failed to perform operation")
        }

        pub async fn input(&self, input: Vec<u8>) -> anyhow::Result<()> {
            self.0
                .send(ClientToServerMessage::Input(input))
                .context("failed to send input message")
        }

        pub async fn shutdown(&self) -> anyhow::Result<()> {
            let (tx, rx) = oneshot::channel();
            self.0
                .send(ClientToServerMessage::Shutdown(tx))
                .context("failed to send shutdown message")?;
            rx.await
                .context("tx dropped")?
                .context("failed to shutdown")
        }
    }

    async fn client_reader_task(
        mut reader: crate::ClientReader,
        s2c_tx: broadcast::Sender<ServerToClientMessage>,
    ) -> anyhow::Result<()> {
        loop {
            match reader.recv().await {
                Ok(value) => {
                    s2c_tx.send(value).ok();
                }
                Err(error) => {
                    tracing::error!(?error, "failed to receive message from client");
                }
            }
        }
    }

    async fn client_writer_task_handle_message(
        message: ClientToServerMessage,
        writer: &mut crate::ClientWriter,
        reader: &mut ClientReader,
    ) -> anyhow::Result<()> {
        match message {
            ClientToServerMessage::UpdateConfig(config, rx) => {
                let task_id = writer
                    .update_config(config)
                    .await
                    .context("failed to send update config message")?;
                let ServerToClientMessage::ConfigUpdated(..) = reader
                    .expect(|m| m.task_id() == Some(task_id))
                    .await
                    .context("failed to receive config updated message")?
                else {
                    anyhow::bail!("got unexpected s2c message, expected ConfigUpdated");
                };
                rx.send(()).ok();
                Ok(())
            }
            ClientToServerMessage::PerformOperation(operation, rx) => {
                let task_id = writer
                    .perform_operation(operation)
                    .await
                    .context("failed to send perform operation message")?;
                let message = reader
                    .expect(|m| m.task_id() == Some(task_id))
                    .await
                    .context("failed to receive operation performed message")?;

                match message {
                    ServerToClientMessage::OperationPerformed(..) => {
                        rx.send(Ok(())).ok();
                    }
                    ServerToClientMessage::OperationFailed(_, _, error, _) => {
                        rx.send(Err(error.into())).ok();
                    }
                    _ => {
                        anyhow::bail!(
                            "got unexpected s2c message, expected OperationPerformed or OperationFailed"
                        );
                    }
                }

                Ok(())
            }
            ClientToServerMessage::Input(input) => writer
                .input(input)
                .await
                .context("failed to send input message"),
            ClientToServerMessage::Shutdown(tx) => {
                if !writer.is_unix() {
                    writer
                        .shutdown()
                        .await
                        .context("failed to send shutdown message")?;
                    tx.send(Ok(())).ok();
                } else {
                    tx.send(Err(NotALocalClient)).ok();
                }

                Ok(())
            }
        }
    }

    async fn client_writer_task(
        mut writer: crate::ClientWriter,
        mut reader: ClientReader,
        mut c2s_rx: UnboundedReceiver<ClientToServerMessage>,
    ) -> anyhow::Result<()> {
        loop {
            match c2s_rx.recv().await {
                Some(message) => {
                    if let Err(error) =
                        client_writer_task_handle_message(message, &mut writer, &mut reader).await
                    {
                        tracing::error!(?error, "failed to send message to server: {error:#}");
                    }
                }
                None => break Ok(()),
            }
        }
    }

    pub async fn manage(
        reader: crate::ClientReader,
        writer: crate::ClientWriter,
    ) -> (ClientReader, ClientWriter) {
        // note: this check is not enough; what if they are both the same type but come from
        // different sources?
        if (reader.is_unix() && writer.is_tcp()) || (reader.is_tcp() && writer.is_unix()) {
            panic!("mismatched reader and writer");
        }

        let (s2c_tx, s2c_rx) = broadcast::channel(64);
        tokio::spawn(client_reader_task(reader, s2c_tx));

        let client_reader = ClientReader(s2c_rx);

        let (c2s_tx, c2s_rx) = mpsc::unbounded_channel();
        tokio::spawn({
            let reader = client_reader.clone();
            client_writer_task(writer, reader, c2s_rx)
        });

        (client_reader.clone(), ClientWriter(c2s_tx))
    }

    pub async fn from_tcp(addrs: impl ToSocketAddrs) -> io::Result<(ClientReader, ClientWriter)> {
        let (reader, writer) = crate::from_tcp(addrs).await?;
        Ok(manage(reader, writer).await)
    }

    pub async fn from_unix(addr: impl AsRef<Path>) -> io::Result<(ClientReader, ClientWriter)> {
        let (reader, writer) = crate::from_unix(addr).await?;
        Ok(manage(reader, writer).await)
    }
}

pub use managed::manage;

use raphy_protocol::{ClientToServerMessage, Config, Operation, ServerToClientMessage, TaskId};
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
        self.0.write_all(&buf).await.map_err(Into::into)
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
    let stream = TcpStream::connect(addrs).await?;
    let (read_half, write_half) = stream.into_split();

    Ok((
        ClientReader(OwnedReadHalf::Tcp(read_half)),
        ClientWriter(OwnedWriteHalf::Tcp(write_half)),
    ))
}

pub async fn from_unix(addr: impl AsRef<Path>) -> io::Result<(ClientReader, ClientWriter)> {
    let stream = UnixStream::connect(addr).await?;
    let (read_half, write_half) = stream.into_split();

    Ok((
        ClientReader(OwnedReadHalf::Unix(read_half)),
        ClientWriter(OwnedWriteHalf::Unix(write_half)),
    ))
}
