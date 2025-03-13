use crate::base::NetworkToServerMessage;
use anyhow::Context;
use raphy_protocol::{Config, Operation};
use slab::Slab;
use std::sync::Arc;
use std::{env, fs};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc, oneshot};
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle};

const UNIX_SOCKET_PATH: &str = "/tmp/raphy.sock";

pub struct ClientToServerMessage {
    id: usize,
    data: raphy_protocol::ClientToServerMessage,
}

pub struct ServerToClientMessage {
    id: usize,
    data: raphy_protocol::ServerToClientMessage,
}

#[derive(Copy, Clone)]
enum ClientKind {
    Unix,
    Tcp,
}

impl ClientKind {
    fn label(&self) -> &'static str {
        match self {
            ClientKind::Unix => "unix",
            ClientKind::Tcp => "tcp",
        }
    }

    fn stream_label(&self) -> &'static str {
        match self {
            ClientKind::Unix => "unix stream",
            ClientKind::Tcp => "tcp stream",
        }
    }
}

struct Client {
    s2c_tx: UnboundedSender<raphy_protocol::ServerToClientMessage>,
    kind: ClientKind,
}

enum NewClient {
    Unix(UnixStream),
    Tcp(TcpStream),
}

async fn read_subsystem(
    c2s_tx: UnboundedSender<ClientToServerMessage>,
    id: usize,
    mut read_half: impl AsyncRead + Unpin,
    sh: SubsystemHandle<anyhow::Error>,
    kind: ClientKind,
) -> anyhow::Result<()> {
    let kind = kind.stream_label();
    let mut len = None;

    loop {
        let mut buf = vec![0; len.unwrap_or(4)];
        tokio::select! {
            result = read_half.read_exact(&mut buf) => {
                match result {
                    Ok(_) => {
                        if len.is_none() {
                            len = Some(u32::from_le_bytes(buf.try_into().unwrap()) as usize);
                            continue;
                        }

                        match bincode::decode_from_slice::<raphy_protocol::ClientToServerMessage, _>(&buf, bincode::config::standard()) {
                            Ok((data, _)) => {
                                c2s_tx.send(ClientToServerMessage { id, data }).expect("failed to send message to network task");
                            }
                            Err(error) => {
                                tracing::error!("failed to decode message from {kind}: {error}");
                                break;
                            }
                        }

                        len = None;
                    }
                    Err(error) => {
                        tracing::error!("failed to read from {kind}: {error}");
                        break;
                    }
                }
            }
            () = sh.on_shutdown_requested() => break,
        }
    }

    Ok(())
}

async fn write_subsystem(
    mut write_half: impl AsyncWrite + Unpin,
    mut s2c_rx: UnboundedReceiver<raphy_protocol::ServerToClientMessage>,
    sh: SubsystemHandle<anyhow::Error>,
    kind: ClientKind,
) -> anyhow::Result<()> {
    let kind = kind.stream_label();

    loop {
        tokio::select! {
            s2c = s2c_rx.recv() => {
                let Some(s2c) = s2c else { break };
                let data = match bincode::encode_to_vec(s2c, bincode::config::standard()) {
                    Ok(data) => data,
                    Err(error) => {
                        tracing::error!("failed to encode message for {kind}: {error}");
                        break;
                    }
                };
                let mut buf = Vec::with_capacity(4 + data.len());
                buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
                buf.extend(data);

                match write_half.write_all(&buf).await {
                    Ok(_) => {}
                    Err(error) => {
                        tracing::error!("failed to write to {kind}: {error}");
                        break;
                    }
                }
            }
            () = sh.on_shutdown_requested() => break,
        }
    }

    Ok(())
}

struct MessageBroadcaster(Vec<UnboundedSender<raphy_protocol::ServerToClientMessage>>);

impl MessageBroadcaster {
    pub fn broadcast(&self, message: raphy_protocol::ServerToClientMessage) {
        for tx in &self.0 {
            tx.send(message.clone()).unwrap();
        }
    }
}

struct NetworkTask {
    clients: Slab<Client>,
    new_clients_rx: UnboundedReceiver<NewClient>,
    c2s_tx: UnboundedSender<ClientToServerMessage>,
    c2s_rx: UnboundedReceiver<ClientToServerMessage>,
    n2s_tx: UnboundedSender<NetworkToServerMessage>,
    global_s2c_rx: UnboundedReceiver<raphy_protocol::ServerToClientMessage>,
    sh: Option<Arc<SubsystemHandle<anyhow::Error>>>,
}

impl NetworkTask {
    pub fn new(
        new_clients_rx: UnboundedReceiver<NewClient>,
        n2s_tx: UnboundedSender<NetworkToServerMessage>,
        global_s2c_rx: UnboundedReceiver<raphy_protocol::ServerToClientMessage>,
    ) -> Self {
        let (c2s_tx, c2s_rx) = mpsc::unbounded_channel();
        Self {
            clients: Slab::new(),
            new_clients_rx,
            c2s_tx,
            c2s_rx,
            n2s_tx,
            global_s2c_rx,
            sh: None,
        }
    }

    fn sh(&self) -> &SubsystemHandle<anyhow::Error> {
        self.sh
            .as_ref()
            .expect("subsystem handle is not yet initialized")
    }

    fn broadcast_message(&self, message: raphy_protocol::ServerToClientMessage) {
        for (_, client) in &self.clients {
            client.s2c_tx.send(message.clone()).unwrap();
        }
    }

    fn message_broadcaster(&self) -> MessageBroadcaster {
        let txs = self
            .clients
            .iter()
            .map(|(_, client)| client.s2c_tx.clone())
            .collect();
        MessageBroadcaster(txs)
    }

    pub async fn run(mut self, sh: SubsystemHandle<anyhow::Error>) {
        let sh = Arc::new(sh);
        self.sh = Some(Arc::clone(&sh));

        loop {
            tokio::select! {
                Some(new_client) = self.new_clients_rx.recv() => self.handle_new_client(new_client),
                Some(c2s) = self.c2s_rx.recv() => self.handle_c2s(c2s),
                Some(message) = self.global_s2c_rx.recv() => self.broadcast_message(message),
                () = sh.on_shutdown_requested() => break,
            }
        }
    }
}

impl NetworkTask {
    fn handle_new_stream(
        &mut self,
        read_half: impl AsyncRead + Send + Unpin + 'static,
        write_half: impl AsyncWrite + Send + Unpin + 'static,
        kind: ClientKind,
    ) {
        let (s2c_tx, s2c_rx) = mpsc::unbounded_channel();
        let id = self.clients.insert(Client { s2c_tx, kind });
        let c2s_tx = self.c2s_tx.clone();
        self.sh().start(SubsystemBuilder::new(
            format!("{}-read-{id}", kind.label()),
            move |sh| async move { read_subsystem(c2s_tx, id, read_half, sh, kind).await },
        ));
        self.sh().start(SubsystemBuilder::new(
            format!("unix-write-{id}"),
            move |sh| async move { write_subsystem(write_half, s2c_rx, sh, kind).await },
        ));
    }

    fn handle_new_unix_stream(&mut self, client: UnixStream) {
        let (read_half, write_half) = client.into_split();
        self.handle_new_stream(read_half, write_half, ClientKind::Unix);
    }

    fn handle_new_tcp_stream(&mut self, client: TcpStream) {
        let (read_half, write_half) = client.into_split();
        self.handle_new_stream(read_half, write_half, ClientKind::Tcp);
    }

    fn handle_new_client(&mut self, new_client: NewClient) {
        match new_client {
            NewClient::Unix(stream) => self.handle_new_unix_stream(stream),
            NewClient::Tcp(stream) => self.handle_new_tcp_stream(stream),
        }
    }
}

impl NetworkTask {
    fn handle_c2s_update_config(&self, config: Config) {
        let (tx, rx) = oneshot::channel();
        self.n2s_tx
            .send(NetworkToServerMessage::UpdateConfig(config.clone(), tx))
            .unwrap();

        let message_broadcaster = self.message_broadcaster();
        tokio::spawn(async move {
            rx.await.unwrap();
            message_broadcaster
                .broadcast(raphy_protocol::ServerToClientMessage::ConfigUpdated(config))
        });
    }

    fn handle_c2s_perform_operation(&self, operation: Operation) {
        self.broadcast_message(raphy_protocol::ServerToClientMessage::OperationRequested(
            operation,
        ));

        let (tx, rx) = oneshot::channel();
        self.n2s_tx
            .send(NetworkToServerMessage::PerformOperation(operation, tx))
            .unwrap();

        let message_broadcaster = self.message_broadcaster();
        tokio::spawn(async move {
            let message = match rx.await.unwrap() {
                Ok(()) => raphy_protocol::ServerToClientMessage::OperationPerformed(operation),
                Err(error) => todo!(),
            };

            message_broadcaster.broadcast(message);
        });
    }

    fn handle_c2s_input(&self, input: Vec<u8>) {
        self.n2s_tx
            .send(NetworkToServerMessage::Input(input))
            .unwrap();
    }

    fn handle_c2s_shutdown(&self, id: usize) {
        let Some(client) = self.clients.get(id) else {
            tracing::warn!("client {id} tried to shut down the server, but it doesn't exist",);
            return;
        };

        if !matches!(client.kind, ClientKind::Unix) {
            tracing::warn!(
                "client {id} tried to shut down the server, but it's not a remote client",
            );
        }

        self.n2s_tx.send(NetworkToServerMessage::Shutdown).unwrap()
    }

    fn handle_c2s(&self, c2s: ClientToServerMessage) {
        match c2s.data {
            raphy_protocol::ClientToServerMessage::UpdateConfig(config) => {
                self.handle_c2s_update_config(config)
            }
            raphy_protocol::ClientToServerMessage::PerformOperation(operation) => {
                self.handle_c2s_perform_operation(operation)
            }
            raphy_protocol::ClientToServerMessage::Input(input) => self.handle_c2s_input(input),
            raphy_protocol::ClientToServerMessage::Shutdown => self.handle_c2s_shutdown(c2s.id),
        }
    }
}

async fn unix(
    new_clients: UnboundedSender<NewClient>,
    sh: SubsystemHandle<anyhow::Error>,
) -> anyhow::Result<()> {
    let listener = UnixListener::bind(UNIX_SOCKET_PATH)
        .with_context(|| format!("Failed to bind unix socket path '{UNIX_SOCKET_PATH}'."))?;
    tracing::info!("listening on unix socket '{UNIX_SOCKET_PATH}'");

    loop {
        tokio::select! {
            result = listener.accept() => {
                let stream = match result {
                   Ok((stream, _)) => stream,
                   Err(error) => {
                       tracing::error!("failed to accept incoming connection from unix socket: {error}");
                       continue;
                   }
                };

                new_clients.send(NewClient::Unix(stream))
                    .expect("failed to send new unix client to network task");
            }
            () = sh.on_shutdown_requested() => {
                drop(listener);

                if let Err(error) = fs::remove_file(UNIX_SOCKET_PATH) {
                    tracing::error!("failed to remove unix socket path '{UNIX_SOCKET_PATH}': {error}");
                }

                return Ok(())
            }
        }
    }
}

async fn tcp(
    address: String,
    new_clients: UnboundedSender<NewClient>,
    port_tx: oneshot::Sender<u16>,
    sh: SubsystemHandle<anyhow::Error>,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&address)
        .await
        .with_context(|| format!("Failed to bind TCP listener to address `{address}`."))?;
    let local_addr = listener
        .local_addr()
        .context("Failed to get local address of TCP listener.")?;
    tracing::info!("listening on tcp address {local_addr}");
    port_tx.send(local_addr.port()).unwrap();

    loop {
        tokio::select! {
            result = listener.accept() => {
                let stream = match result {
                    Ok((stream, _)) => stream,
                    Err(error) => {
                        tracing::error!("failed to accept incoming connection from TCP listener: {error}");
                        continue;
                    }
                };

                new_clients.send(NewClient::Tcp(stream))
                    .expect("failed to send new TCP client to network task");
            }
            () = sh.on_shutdown_requested() => break,
        }
    }

    Ok(())
}

pub async fn initialize(
    sh: &SubsystemHandle<anyhow::Error>,
    n2s_tx: UnboundedSender<NetworkToServerMessage>,
    global_s2c_rx: UnboundedReceiver<raphy_protocol::ServerToClientMessage>,
) -> anyhow::Result<u16> {
    let address = env::var("RAPHY_SERVER_ADDRESS").unwrap_or_else(|_| "0.0.0.0:0".to_owned());
    let (new_clients_tx, new_clients_rx) = mpsc::unbounded_channel();

    sh.start(SubsystemBuilder::new("unix-listener", {
        let new_clients_tx = new_clients_tx.clone();
        move |sh| unix(new_clients_tx, sh)
    }));

    let (port_tx, port_rx) = oneshot::channel();
    sh.start(SubsystemBuilder::new("tcp-listener", move |sh| {
        tcp(address, new_clients_tx, port_tx, sh)
    }));

    let network = NetworkTask::new(new_clients_rx, n2s_tx, global_s2c_rx);
    sh.start(SubsystemBuilder::new("network", move |sh| async move {
        network.run(sh).await;
        Ok::<_, anyhow::Error>(())
    }));

    Ok(port_rx.await.expect("port tx was dropped"))
}
