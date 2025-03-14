use crate::base::NetworkToServerMessage;
use anyhow::Context;
use raphy_protocol::{Config, Operation, OperationId, SerdeError, TaskId};
use slab::Slab;
use std::collections::HashMap;
use std::sync::Arc;
use std::{env, fmt, fs};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc, oneshot};
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle};

const UNIX_SOCKET_PATH: &str = "/tmp/raphy.sock";

#[derive(Copy, Clone)]
pub struct ClientId(usize);

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub struct ClientToServerMessage {
    id: ClientId,
    data: raphy_protocol::ClientToServerMessage,
}

pub struct ServerToClientMessage {
    id: ClientId,
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
    id: ClientId,
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

struct MessageBroadcaster {
    senders: Vec<UnboundedSender<raphy_protocol::ServerToClientMessage>>,
    active_task: Option<(
        TaskId,
        UnboundedSender<raphy_protocol::ServerToClientMessage>,
    )>,
}

impl MessageBroadcaster {
    pub fn broadcast(self, message: raphy_protocol::ServerToClientMessage) {
        if let Some((_, tx)) = self.active_task {
            tx.send(message.clone()).unwrap();
        }

        for tx in self.senders {
            tx.send(message.clone()).unwrap();
        }
    }

    pub fn broadcast_with_task_id(
        self,
        mut message_fn: impl FnMut(Option<TaskId>) -> raphy_protocol::ServerToClientMessage,
    ) {
        if let Some((task_id, tx)) = self.active_task {
            tx.send(message_fn(Some(task_id))).unwrap();
        }

        for tx in &self.senders {
            tx.send(message_fn(None)).unwrap();
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

    fn message_broadcaster(&self, active_task: Option<(ClientId, TaskId)>) -> MessageBroadcaster {
        if let Some((client_id, task_id)) = active_task {
            let mut senders: HashMap<_, _> = self
                .clients
                .iter()
                .map(|(cid, c)| (cid, c.s2c_tx.clone()))
                .collect();
            let active_task = senders.remove(&client_id.0).map(|tx| (task_id, tx));

            MessageBroadcaster {
                senders: senders.into_iter().map(|(_, tx)| tx).collect(),
                active_task,
            }
        } else {
            MessageBroadcaster {
                senders: self.clients.iter().map(|(_, c)| c.s2c_tx.clone()).collect(),
                active_task: None,
            }
        }
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
        let id = ClientId(self.clients.insert(Client { s2c_tx, kind }));
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
    fn handle_c2s_update_config(&self, client_id: ClientId, task_id: TaskId, config: Config) {
        let (tx, rx) = oneshot::channel();
        self.n2s_tx
            .send(NetworkToServerMessage::UpdateConfig(config.clone(), tx))
            .unwrap();

        let message_broadcaster = self.message_broadcaster(Some((client_id, task_id)));
        tokio::spawn(async move {
            rx.await.unwrap();
            message_broadcaster.broadcast_with_task_id(|tid| {
                raphy_protocol::ServerToClientMessage::ConfigUpdated(config.clone(), tid)
            })
        });
    }

    fn handle_c2s_perform_operation(
        &self,
        client_id: ClientId,
        task_id: TaskId,
        operation: Operation,
    ) {
        let op_id = OperationId::generate();
        self.broadcast_message(raphy_protocol::ServerToClientMessage::OperationRequested(
            operation, op_id,
        ));

        let (tx, rx) = oneshot::channel();
        self.n2s_tx
            .send(NetworkToServerMessage::PerformOperation(operation, tx))
            .unwrap();

        let message_broadcaster = self.message_broadcaster(Some((client_id, task_id)));
        tokio::spawn(async move {
            match rx.await.unwrap() {
                Ok(()) => message_broadcaster.broadcast_with_task_id(|tid| {
                    raphy_protocol::ServerToClientMessage::OperationPerformed(operation, op_id, tid)
                }),
                Err(error) => message_broadcaster.broadcast_with_task_id(|tid| {
                    raphy_protocol::ServerToClientMessage::OperationFailed(
                        operation,
                        op_id,
                        SerdeError::new(&*error),
                        tid,
                    )
                }),
            }
        });
    }

    fn handle_c2s_input(&self, input: Vec<u8>) {
        self.n2s_tx
            .send(NetworkToServerMessage::Input(input))
            .unwrap();
    }

    fn handle_c2s_shutdown(&self, id: ClientId) {
        let Some(client) = self.clients.get(id.0) else {
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
            raphy_protocol::ClientToServerMessage::UpdateConfig(task_id, config) => {
                self.handle_c2s_update_config(c2s.id, task_id, config)
            }
            raphy_protocol::ClientToServerMessage::PerformOperation(task_id, operation) => {
                self.handle_c2s_perform_operation(c2s.id, task_id, operation)
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
