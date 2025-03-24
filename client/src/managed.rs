use anyhow::Context;
use raphy_protocol::{Config, Operation, ServerToClientMessage};
use std::io;
use std::path::Path;
use thiserror::Error;
use tokio::net::ToSocketAddrs;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

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
            tracing::debug!(?message);

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
    Ping(oneshot::Sender<()>),
    GetConfig(oneshot::Sender<Option<Config>>),
    UpdateConfig(Config, oneshot::Sender<()>),
    PerformOperation(Operation, oneshot::Sender<anyhow::Result<()>>),
    Input(Vec<u8>),
    Shutdown(oneshot::Sender<Result<(), NotALocalClient>>),
}

#[derive(Clone)]
pub struct ClientWriter(UnboundedSender<ClientToServerMessage>);

impl ClientWriter {
    pub async fn ping(&self) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.0
            .send(ClientToServerMessage::Ping(tx))
            .context("c2s channel closed")?;
        rx.await.context("tx dropped")
    }
    
    pub async fn get_config(&self) -> anyhow::Result<Option<Config>> {
        let (tx, rx) = oneshot::channel();
        self.0
            .send(ClientToServerMessage::GetConfig(tx))
            .context("c2s channel closed")?;
        rx.await.context("tx dropped")
    }

    pub async fn update_config(&self, config: Config) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.0
            .send(ClientToServerMessage::UpdateConfig(config, tx))
            .context("c2s channel closed")?;
        rx.await.context("tx dropped")
    }

    pub async fn perform_operation(&self, operation: Operation) -> anyhow::Result<()> {
        tracing::info!("managed perform operation");
        let (tx, rx) = oneshot::channel();
        self.0
            .send(ClientToServerMessage::PerformOperation(operation, tx))
            .context("c2s channel closed")?;
        rx.await
            .context("tx dropped")?
            .context("failed to perform operation")
    }

    pub async fn input(&self, input: Vec<u8>) -> anyhow::Result<()> {
        self.0
            .send(ClientToServerMessage::Input(input))
            .context("c2s channel closed")
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.0
            .send(ClientToServerMessage::Shutdown(tx))
            .context("c2s channel closed")?;
        rx.await
            .context("tx dropped")?
            .context("failed to shutdown")
    }
}

async fn client_reader_task(
    mut reader: crate::ClientReader,
    s2c_tx: broadcast::Sender<ServerToClientMessage>,
    cancel_token: CancellationToken,
) -> anyhow::Result<()> {
    tracing::debug!("begin client reader task");

    loop {
        tokio::select! {
            result = reader.recv() => match result {
                Ok(value) => {
                    s2c_tx.send(value).ok();
                }
                Err(error) => {
                    tracing::error!(?error, "failed to receive message from client");
                    cancel_token.cancel()
                }
            },
            () = cancel_token.cancelled() => break Ok(()),
        }
    }
}

async fn client_writer_task_handle_message(
    message: ClientToServerMessage,
    writer: &mut crate::ClientWriter,
    reader: &mut ClientReader,
) -> anyhow::Result<()> {
    match message {
        ClientToServerMessage::Ping(rx) => {
            tracing::debug!("receive ping");
            let task_id = writer.ping().await.context("failed to send ping message")?;
            let ServerToClientMessage::Pong(..) = reader
                .expect(|m| m.task_id() == Some(task_id))
                .await
                .context("failed to receive pong message")?
            else {
                anyhow::bail!("got unexpected s2c message, expected Pong");
            };
            rx.send(()).ok();
            Ok(())
        }
        ClientToServerMessage::GetConfig(rx) => {
            let task_id = writer
                .get_config()
                .await
                .context("failed to send get config message")?;
            let ServerToClientMessage::CurrentConfig(config, ..) = reader
                .expect(|m| m.task_id() == Some(task_id))
                .await
                .context("failed to receive current config message")?
            else {
                anyhow::bail!("got unexpected s2c message, expected CurrentConfig");
            };
            rx.send(config).ok();
            Ok(())
        }
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
    cancel_token: CancellationToken,
) -> anyhow::Result<()> {
    tracing::debug!("begin client writer task");

    loop {
        tokio::select! {
            result = c2s_rx.recv() => match result {
                Some(message) => {
                    if let Err(error) =
                        client_writer_task_handle_message(message, &mut writer, &mut reader).await
                    {
                        tracing::error!(?error, "failed to send message to server: {error:#}");
                        cancel_token.cancel();
                    }
                }
                None => cancel_token.cancel(),
            },
            () = cancel_token.cancelled() => break Ok(()),
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

    let cancel_token = CancellationToken::new();

    let (s2c_tx, s2c_rx) = broadcast::channel(64);
    tokio::spawn(client_reader_task(reader, s2c_tx, cancel_token.clone()));

    let client_reader = ClientReader(s2c_rx);

    let (c2s_tx, c2s_rx) = mpsc::unbounded_channel();
    tokio::spawn({
        let reader = client_reader.clone();
        client_writer_task(writer, reader, c2s_rx, cancel_token)
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
