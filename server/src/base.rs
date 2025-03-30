use crate::child;
use crate::child::ServerToChildMessage;
use raphy_protocol::{Config, Operation, ServerState};
use std::process::ExitStatus;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio_graceful_shutdown::SubsystemHandle;

pub enum NetworkToServerMessage {
    GetConfig(oneshot::Sender<Option<Config>>),
    GetServerState(oneshot::Sender<ServerState>),
    UpdateConfig(Config, oneshot::Sender<()>),
    PerformOperation(Operation, oneshot::Sender<anyhow::Result<()>>),
    Input(Vec<u8>),
    Shutdown,
}

pub enum ChildToServerMessage {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    UpdateState(ServerState),
}

pub struct ServerTask {
    config: Option<Config>,
    n2s_rx: UnboundedReceiver<NetworkToServerMessage>,
    ch2s_rx: UnboundedReceiver<ChildToServerMessage>,
    s2ch_tx: UnboundedSender<ServerToChildMessage>,
    global_s2c_tx: UnboundedSender<raphy_protocol::ServerToClientMessage>,
    sh: Option<Arc<SubsystemHandle<anyhow::Error>>>,
}

impl ServerTask {
    pub fn new(
        n2s_rx: UnboundedReceiver<NetworkToServerMessage>,
        ch2s_rx: UnboundedReceiver<ChildToServerMessage>,
        s2ch_tx: UnboundedSender<ServerToChildMessage>,
        global_s2c_tx: UnboundedSender<raphy_protocol::ServerToClientMessage>,
        config: Option<Config>,
    ) -> Self {
        Self {
            config,
            n2s_rx,
            ch2s_rx,
            s2ch_tx,
            global_s2c_tx,
            sh: None,
        }
    }

    fn sh(&self) -> &SubsystemHandle<anyhow::Error> {
        self.sh
            .as_ref()
            .expect("subsystem handle is not yet initialized")
    }

    async fn handle_n2s(&mut self, message: NetworkToServerMessage) {
        match message {
            NetworkToServerMessage::GetConfig(ret) => {
                ret.send(self.config.clone()).ok().unwrap();
            }
            NetworkToServerMessage::GetServerState(ret) => {
                self.s2ch_tx.send(ServerToChildMessage::ServerState(ret)).ok().unwrap();
            }
            NetworkToServerMessage::UpdateConfig(config, ret) => {
                if let Err(error) = config.dump().await {
                    tracing::error!(?error, "failed to save the configuration: {error:#}");
                }

                self.config = Some(config.clone());
                self.s2ch_tx
                    .send(ServerToChildMessage::UpdateConfig(config))
                    .unwrap();
                ret.send(()).unwrap()
            }
            NetworkToServerMessage::PerformOperation(operation, ret) => match operation {
                Operation::Start => self.s2ch_tx.send(ServerToChildMessage::Start(ret)).unwrap(),
                Operation::Stop => {
                    self.s2ch_tx.send(ServerToChildMessage::Stop(ret)).unwrap();
                }
                Operation::Restart => self
                    .s2ch_tx
                    .send(ServerToChildMessage::Restart(ret))
                    .unwrap(),
            },
            NetworkToServerMessage::Input(input) => self
                .s2ch_tx
                .send(ServerToChildMessage::Stdin(input))
                .unwrap(),
            NetworkToServerMessage::Shutdown => self.sh().request_shutdown(),
        }
    }

    fn handle_ch2s(&self, message: ChildToServerMessage) {
        match message {
            ChildToServerMessage::Stdout(out) => {
                self.global_s2c_tx
                    .send(raphy_protocol::ServerToClientMessage::Stdout(out))
                    .ok();
            }
            ChildToServerMessage::Stderr(err) => {
                self.global_s2c_tx
                    .send(raphy_protocol::ServerToClientMessage::Stderr(err))
                    .ok();
            }
            ChildToServerMessage::UpdateState(state) => {
                self.global_s2c_tx
                    .send(raphy_protocol::ServerToClientMessage::ServerStateUpdated(
                        state,
                    ))
                    .ok();
            }
        }
    }

    pub async fn run(mut self, sh: SubsystemHandle<anyhow::Error>) {
        let sh = Arc::new(sh);
        self.sh = Some(Arc::clone(&sh));

        loop {
            tokio::select! {
                Some(message) = self.n2s_rx.recv() => self.handle_n2s(message).await,
                Some(message) = self.ch2s_rx.recv() => self.handle_ch2s(message),
                () = sh.on_shutdown_requested() => break,
            }
        }
    }
}
