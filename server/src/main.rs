mod network;
mod child;
mod base {
    use crate::child::ServerToChildMessage;
    use raphy_protocol::{Config, Operation};
    use std::process::ExitStatus;
    use std::sync::Arc;
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
    use tokio::sync::oneshot;
    use tokio_graceful_shutdown::SubsystemHandle;

    pub enum NetworkToServerMessage {
        UpdateConfig(Config, oneshot::Sender<()>),
        PerformOperation(Operation, oneshot::Sender<anyhow::Result<()>>),
        Input(Vec<u8>),
        Shutdown,
    }

    pub enum ChildToServerMessage {
        Stdout(Vec<u8>),
        Stderr(Vec<u8>),
        UnexpectedExit(Option<ExitStatus>),
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

        fn handle_n2s(&mut self, message: NetworkToServerMessage) {
            match message {
                NetworkToServerMessage::UpdateConfig(config, ret) => {
                    self.config = Some(config.clone());
                    self.s2ch_tx
                        .send(ServerToChildMessage::UpdateConfig(config))
                        .unwrap();
                    ret.send(()).unwrap()
                }
                NetworkToServerMessage::PerformOperation(operation, ret) => {
                    match operation {
                        Operation::Start => {
                            self.s2ch_tx.send(ServerToChildMessage::Start(ret)).unwrap()
                        }
                        Operation::Stop => {
                            self.s2ch_tx.send(ServerToChildMessage::Stop(ret)).unwrap();
                        }
                        Operation::Restart => self
                            .s2ch_tx
                            .send(ServerToChildMessage::Restart(ret))
                            .unwrap(),
                    }
                }
                NetworkToServerMessage::Input(input) => self
                    .s2ch_tx
                    .send(ServerToChildMessage::Stdin(input))
                    .unwrap(),
                NetworkToServerMessage::Shutdown => self.sh().request_shutdown(),
            }
        }
        
        fn handle_ch2s(&self, message: ChildToServerMessage) {
            match message {
                ChildToServerMessage::Stdout(out) => self.global_s2c_tx.send(raphy_protocol::ServerToClientMessage::Stdout(out)).unwrap(),
                ChildToServerMessage::Stderr(err) => self.global_s2c_tx.send(raphy_protocol::ServerToClientMessage::Stderr(err)).unwrap(),
                ChildToServerMessage::UnexpectedExit(exit_status) => todo!(),
            }
        }

        pub async fn run(mut self, sh: SubsystemHandle<anyhow::Error>) {
            let sh = Arc::new(sh);
            self.sh = Some(Arc::clone(&sh));

            loop {
                tokio::select! {
                    Some(message) = self.n2s_rx.recv() => self.handle_n2s(message),
                    Some(message) = self.ch2s_rx.recv() => self.handle_ch2s(message),
                    () = sh.on_shutdown_requested() => break,
                }
            }
        }
    }
}
mod utils {
    use anyhow::Context;
    use mdns_sd::{ServiceDaemon, ServiceInfo};

    pub fn start_advertising(port: u16) -> anyhow::Result<()> {
        tracing::info!("create mdns service daemon");
        let mdns = ServiceDaemon::new().context("Failed to create mDNS service daemon.")?;
        let service_info = ServiceInfo::new(
            raphy_protocol::SERVICE_TYPE,
            raphy_protocol::INSTANCE_NAME,
            &format!(
                "{}.{}",
                raphy_protocol::INSTANCE_NAME,
                raphy_protocol::SERVICE_TYPE
            ),
            "",
            port,
            None,
        )
        .expect("service info was invalid")
        .enable_addr_auto();

        tracing::info!("register service info with mdns");
        mdns.register(service_info)
            .context("Failed to register service info with mDNS.")?;

        Ok(())
    }
}

use anyhow::Context;
use native_dialog::MessageType;
use std::fmt::{Debug, Display};
use std::process::ExitCode;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};
use raphy_protocol::Config;
use crate::child::ChildTask;

async fn real_main(sh: SubsystemHandle<anyhow::Error>) -> anyhow::Result<()> {
    let (n2s_tx, n2s_rx) = mpsc::unbounded_channel();
    let (global_s2c_tx, global_s2c_rx) = mpsc::unbounded_channel();
    let port = network::initialize(&sh, n2s_tx, global_s2c_rx)
        .await
        .context("Failed to initialize the network subsystem.")?;

    utils::start_advertising(port).context("Failed to start advertising mDNS service.")?;

    let config = Config::load().await.context("Failed to load the server configuration.")?;
    let (s2ch_tx, s2ch_rx) = mpsc::unbounded_channel();
    let (ch2s_tx, ch2s_rx) = mpsc::unbounded_channel();
    let child_task = ChildTask::new(s2ch_rx, ch2s_tx, config.clone());
    
    sh.start(SubsystemBuilder::new("child", move |sh| async move {
        child_task.run(sh).await;
        Ok::<_, anyhow::Error>(())
    }));
    
    let server_task = base::ServerTask::new(n2s_rx, ch2s_rx, s2ch_tx, global_s2c_tx, config);
    sh.start(SubsystemBuilder::new("server", move |sh| async move {
        server_task.run(sh).await;
        Ok::<_, anyhow::Error>(())
    }));

    Ok(())
}

async fn handle_error(error: impl Display + Debug + Send + Sync + 'static) {
    tracing::error!(?error, "{error:#}");

    tokio::task::spawn_blocking(move || {
        if let Err(error) = native_dialog::MessageDialog::new()
            .set_title("raphy server application crashed.")
            .set_text(&format!("One or more errors occurred.\n\n{error:?}"))
            .set_type(MessageType::Error)
            .show_alert()
        {
            tracing::error!("failed to show error dialog: {error}");
        }
    })
    .await
    .unwrap()
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();

    if let Err(error) = Toplevel::new(|sh| async move {
        if let Err(error) = real_main(sh).await {
            handle_error(error).await
        }
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_secs(60))
    .await
    {
        handle_error(error).await;
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
