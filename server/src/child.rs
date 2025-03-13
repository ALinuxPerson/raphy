use crate::base::ChildToServerMessage;
use anyhow::Context;
use raphy_protocol::Config;
use std::io;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{mpsc, oneshot};
use tokio_graceful_shutdown::{NestedSubsystem, SubsystemBuilder, SubsystemHandle};

pub enum ServerToChildMessage {
    Stdin(Vec<u8>),
    Start(oneshot::Sender<anyhow::Result<()>>),
    Stop(oneshot::Sender<anyhow::Result<()>>),
    Restart(oneshot::Sender<anyhow::Result<()>>),
    UpdateConfig(Config),
}

enum State {
    Running {
        std: NestedSubsystem<anyhow::Error>,
        waiter_rx: Option<oneshot::Receiver<io::Result<ExitStatus>>>,
        stdin_tx: UnboundedSender<Vec<u8>>,
    },
    Stopped,
}

pub struct ChildTask {
    state: State,
    s2c_rx: UnboundedReceiver<ServerToChildMessage>,
    c2s_tx: UnboundedSender<ChildToServerMessage>,
    config: Option<Config>,
    sh: Option<Arc<SubsystemHandle<anyhow::Error>>>,
}

impl ChildTask {
    pub fn new(
        s2c_rx: UnboundedReceiver<ServerToChildMessage>,
        c2s_tx: UnboundedSender<ChildToServerMessage>,
        config: Option<Config>,
    ) -> Self {
        Self {
            state: State::Stopped,
            s2c_rx,
            c2s_tx,
            config,
            sh: None,
        }
    }

    fn sh(&self) -> &SubsystemHandle<anyhow::Error> {
        self.sh
            .as_ref()
            .expect("subsystem handle is not yet initialized")
    }

    pub async fn run(mut self, sh: SubsystemHandle<anyhow::Error>) {
        let sh = Arc::new(sh);
        self.sh = Some(Arc::clone(&sh));

        loop {
            tokio::select! {
                Some(message) = self.s2c_rx.recv() => self.handle_s2c(message).await,
                () = sh.on_shutdown_requested() => break,
            }
        }
    }
}

async fn output_subsystem(
    mut reader: impl AsyncRead + Unpin,
    tx: UnboundedSender<Vec<u8>>,
    sh: SubsystemHandle<anyhow::Error>,
    std: &'static str,
) -> anyhow::Result<()> {
    loop {
        let mut buffer = vec![0; 1024];
        let n = tokio::select! {
            result = reader.read(&mut buffer) => match result {
                Ok(n) => n,
                Err(error) => {
                    tracing::error!("failed to read from {std}: {error}");
                    continue
                }
            },
            () = sh.on_shutdown_requested() => break,
        };

        buffer.truncate(n);
        tx.send(buffer).unwrap();
    }

    Ok(())
}

impl ChildTask {
    fn handle_s2c_stdin(&mut self, input: Vec<u8>) {
        if let State::Running { stdin_tx, .. } = &self.state {
            stdin_tx.send(input).unwrap();
        }
    }

    async fn handle_s2c_start(&mut self) -> anyhow::Result<()> {
        if matches!(self.state, State::Running { .. }) {
            return Ok(());
        }

        let Some(config) = &self.config else {
            anyhow::bail!("A server configuration is required to start the server.");
        };
        let java_path = config
            .java_path
            .get()
            .context("Failed to get the Java path.")?;
        let args = config
            .arguments
            .get()
            .context("Failed to get the server arguments.")?;
        let mut command = match config.user.make_command() {
            Some(mut command) => {
                command.arg(&*java_path);
                command
            }
            None => Command::new(&*java_path),
        };
        let mut child = command
            .arg("-jar")
            .arg(&config.server_jar_path)
            .args(args.iter())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start the server process.")?;

        let c2s_tx = self.c2s_tx.clone();
        let (stdin_tx_tx, stdin_tx_rx) = oneshot::channel();
        let mut stdin = child
            .stdin
            .take()
            .expect("child did not have a handle to stdin");
        let stdout = child
            .stdout
            .take()
            .expect("child did not have a handle to stdout");
        let stderr = child
            .stderr
            .take()
            .expect("child did not have a handle to stderr");
        let root = self.sh().start(SubsystemBuilder::new("std", |sh| async move {
            let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();
            stdin_tx_tx.send(stdin_tx).unwrap();
            sh.start(SubsystemBuilder::new("in", |sh| async move {
                loop {
                    tokio::select! {
                            Some(input) = stdin_rx.recv() => {
                                if let Err(error) = stdin.write_all(&input).await {
                                    tracing::error!("failed to write to stdin: {error}");
                                }
                            },
                            () = sh.on_shutdown_requested() => break,
                        }
                }

                Ok::<_, anyhow::Error>(())
            }));

            let (stdout_tx, mut stdout_rx) = mpsc::unbounded_channel();
            sh.start(SubsystemBuilder::new("out", |sh| async move {
                output_subsystem(stdout, stdout_tx, sh, "stdout").await
            }));

            let (stderr_tx, mut stderr_rx) = mpsc::unbounded_channel();
            sh.start(SubsystemBuilder::new("err", |sh| async move {
                output_subsystem(stderr, stderr_tx, sh, "stderr").await
            }));

            sh.start(SubsystemBuilder::new("helper", |sh| async move {
                loop {
                    tokio::select! {
                        Some(buf) = stdout_rx.recv() => c2s_tx.send(ChildToServerMessage::Stdout(buf)).unwrap(),
                        Some(buf) = stderr_rx.recv() => c2s_tx.send(ChildToServerMessage::Stderr(buf)).unwrap(),
                        () = sh.on_shutdown_requested() => break,
                    }
                }

                Ok::<_, anyhow::Error>(())
            }));

            Ok::<_, anyhow::Error>(())
        }));

        let (waiter_tx, waiter_rx) = oneshot::channel();
        self.sh()
            .start(SubsystemBuilder::new("waiter", |sh| async move {
                waiter_tx.send(child.wait().await).unwrap();
                Ok::<_, anyhow::Error>(())
            }));

        self.state = State::Running {
            std: root,
            waiter_rx: Some(waiter_rx),
            stdin_tx: stdin_tx_rx.await.unwrap(),
        };

        Ok(())
    }

    async fn handle_s2c_stop(&mut self) {
        if let State::Running {
            std, waiter_rx, ..
        } = &mut self.state
        {
            std.initiate_shutdown();

            if let Some(waiter_rx) = waiter_rx.take() {
                match waiter_rx.await.unwrap() {
                    Ok(exit_status) => {
                        tracing::info!("server process exited with status code {exit_status}");

                        if !exit_status.success() {
                            self.c2s_tx
                                .send(ChildToServerMessage::UnexpectedExit(Some(exit_status)))
                                .unwrap();
                        }
                    }
                    Err(error) => {
                        tracing::error!(
                            "failed to wait for the server process to exit: {error}"
                        );
                        self.c2s_tx
                            .send(ChildToServerMessage::UnexpectedExit(None))
                            .unwrap()
                    }
                }
            }
        }
        self.state = State::Stopped;
    }

    async fn handle_s2c_restart(&mut self) -> anyhow::Result<()> {
        self.handle_s2c_stop().await;
        self.handle_s2c_start().await
    }

    async fn handle_s2c(&mut self, message: ServerToChildMessage) {
        match message {
            ServerToChildMessage::Stdin(input) => self.handle_s2c_stdin(input),
            ServerToChildMessage::Start(ret) => {
                ret.send(self.handle_s2c_start().await).unwrap();
            }
            ServerToChildMessage::Stop(ret) => {
                self.handle_s2c_stop().await;
                ret.send(Ok(())).unwrap()
            }
            ServerToChildMessage::Restart(ret) => {
                ret.send(self.handle_s2c_restart().await).unwrap()
            }
            ServerToChildMessage::UpdateConfig(config) => self.config = Some(config),
        }
    }
}
