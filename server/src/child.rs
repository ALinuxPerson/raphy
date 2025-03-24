use crate::base::ChildToServerMessage;
use anyhow::Context;
use raphy_protocol::{Config, ServerState};
use std::io;
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
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
        stdin_tx: UnboundedSender<Vec<u8>>,
    },
    Stopped,
}

pub struct ChildTask {
    state: State,
    s2c_rx: UnboundedReceiver<ServerToChildMessage>,
    c2s_tx: UnboundedSender<ChildToServerMessage>,
    dead_tx: UnboundedSender<()>,
    dead_rx: UnboundedReceiver<()>,
    config: Option<Config>,
    sh: Option<Arc<SubsystemHandle<anyhow::Error>>>,
}

impl ChildTask {
    pub fn new(
        s2c_rx: UnboundedReceiver<ServerToChildMessage>,
        c2s_tx: UnboundedSender<ChildToServerMessage>,
        config: Option<Config>,
    ) -> Self {
        let (dead_tx, dead_rx) = mpsc::unbounded_channel();
        Self {
            state: State::Stopped,
            s2c_rx,
            c2s_tx,
            dead_tx,
            dead_rx,
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
                Ok(0) => {
                    sh.on_shutdown_requested().await;
                    break
                }
                Ok(n) => n,
                Err(error) => {
                    tracing::error!("failed to read from {std}: {error}");
                    sh.request_local_shutdown();
                    break
                }
            },
            () = sh.on_shutdown_requested() => break,
        };

        tx.send(buffer[..n].to_vec()).ok();
    }

    Ok(())
}

impl ChildTask {
    fn handle_s2c_stdin(&mut self, input: Vec<u8>) {
        if let State::Running { stdin_tx, .. } = &self.state {
            stdin_tx.send(input).unwrap();
        }
    }

    fn handle_s2c_start(&mut self) -> anyhow::Result<()> {
        if matches!(self.state, State::Running { .. }) {
            return Ok(());
        }

        let Some(config) = &self.config else {
            anyhow::bail!("A server configuration is required to start the server.");
        };
        let java_path = config
            .java_path
            .resolve()
            .context("Failed to get the Java path.")?;
        let args = config
            .arguments
            .resolve()
            .context("Failed to get the server arguments.")?;
        let mut command = match config.user.make_command() {
            Some(mut command) => {
                command.arg(&*java_path);
                command
            }
            None => Command::new(&*java_path),
        };
        
        let child = command
            .arg("-jar")
            .arg(&config.server_jar_path)
            .args(args.iter())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        let child_std = child.as_std();
        tracing::debug!(program = ?child_std.get_program(), args = ?child_std.get_args(), "starting server process");

        let mut child = command
            .spawn()
            .context("Failed to start the server process.")?;

        let c2s_tx = self.c2s_tx.clone();
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
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let root = self.sh().start(SubsystemBuilder::new("std", |sh| async move {
            sh.start(SubsystemBuilder::new("in", {
                |sh| async move {
                    loop {
                        tokio::select! {
                            Some(input) = stdin_rx.recv() => {
                                if let Err(error) = stdin.write_all(&input).await {
                                    tracing::error!("failed to write to stdin: {error}");
                                    sh.request_local_shutdown();
                                    break
                                }
                            },
                            () = sh.on_shutdown_requested() => break,
                        }
                    }

                    Ok::<_, anyhow::Error>(())
                }
            }));

            let (stdout_tx, mut stdout_rx) = mpsc::unbounded_channel();
            sh.start(SubsystemBuilder::new("out", |sh| async move {
                output_subsystem(stdout, stdout_tx, sh, "stdout").await
            }));

            let (stderr_tx, mut stderr_rx) = mpsc::unbounded_channel();
            sh.start(SubsystemBuilder::new("err", |sh| async move {
                output_subsystem(stderr, stderr_tx, sh, "stderr").await
            }));

            sh.start(SubsystemBuilder::new("channel-helper", |sh| async move {
                loop {
                    tokio::select! {
                        Some(buf) = stdout_rx.recv() => {
                            c2s_tx.send(ChildToServerMessage::Stdout(buf)).ok();
                        },
                        Some(buf) = stderr_rx.recv() => {
                            c2s_tx.send(ChildToServerMessage::Stderr(buf)).ok();
                        },
                        () = sh.on_shutdown_requested() => break,
                    }
                }

                Ok::<_, anyhow::Error>(())
            }));

            Ok::<_, anyhow::Error>(())
        }));

        let dead_tx = self.dead_tx.clone();
        let c2s_tx = self.c2s_tx.clone();
        self.sh()
            .start(SubsystemBuilder::new("waiter", |sh| async move {
                match child.wait().await {
                    Ok(exit_status) => {
                        tracing::info!("server process exited with status code {exit_status}");

                        c2s_tx
                            .send(ChildToServerMessage::UpdateState(ServerState::Stopped(Some(exit_status.into()))))
                            .ok();
                    }
                    Err(error) => {
                        tracing::error!("failed to wait for the server process to exit: {error}");
                        c2s_tx
                            .send(ChildToServerMessage::UpdateState(ServerState::Stopped(None)))
                            .ok();
                    }
                }
                
                dead_tx.send(()).ok();
                
                Ok::<_, anyhow::Error>(())
            }));

        self.state = State::Running {
            std: root,
            stdin_tx,
        };
        
        self.c2s_tx.send(ChildToServerMessage::UpdateState(ServerState::Started)).ok();

        Ok(())
    }

    async fn handle_s2c_stop(&mut self) {
        if let State::Running { std, .. } = &mut self.state {
            std.initiate_shutdown();
        }
        
        self.state = State::Stopped;
    }

    async fn handle_s2c_restart(&mut self) -> anyhow::Result<()> {
        self.handle_s2c_stop().await;
        self.handle_s2c_start()
    }

    async fn handle_s2c(&mut self, message: ServerToChildMessage) {
        match message {
            ServerToChildMessage::Stdin(input) => self.handle_s2c_stdin(input),
            ServerToChildMessage::Start(ret) => {
                let result = self.handle_s2c_start();

                if let Err(error) = &result {
                    tracing::error!(?error, "failed to start the server: {error:#}")
                }

                ret.send(result).unwrap();
            }
            ServerToChildMessage::Stop(ret) => {
                self.handle_s2c_stop().await;
                ret.send(Ok(())).unwrap()
            }
            ServerToChildMessage::Restart(ret) => {
                let result = self.handle_s2c_restart().await;

                if let Err(error) = &result {
                    tracing::error!(?error, "failed to restart the server: {error:#}")
                }

                ret.send(result).unwrap()
            }
            ServerToChildMessage::UpdateConfig(config) => self.config = Some(config),
        }
    }
}
