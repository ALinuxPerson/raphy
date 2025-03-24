use crate::commands;
use crate::commands::Server;
use anyhow::Context;
use indexmap::IndexMap;
use mdns_sd::ServiceEvent;
use native_dialog::MessageType;
use raphy_client::managed::{ClientReader, ClientWriter};
use raphy_client::ClientMode;
use raphy_protocol::{ServerToClientMessage, UNIX_SOCKET_PATH};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tauri::{App, AppHandle, Emitter, Manager, Wry};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

pub fn emit_message_on_connection_failure(runtime: &Runtime, writer: ClientWriter, app: AppHandle) {
    runtime.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3));
        interval.tick().await;
        
        loop {
            let did_fail = match tokio::time::timeout(Duration::from_secs(30), writer.ping()).await {
                Ok(Ok(())) => false,
                Ok(Err(error)) => {
                    tracing::error!(?error, "failed to send ping message: {error:#}");
                    true
                }
                Err(elapsed) => {
                    tracing::error!("ping timeout: {elapsed:?}");
                    true
                }
            };
            
            if did_fail {
                app.emit("connection-failure", ()).unwrap();
                break;
            } else {
                interval.tick().await;
                continue
            }
        }
    });
}

pub fn emit_message_on_s2c(runtime: &Runtime, mut reader: ClientReader, app: AppHandle) {
    runtime.spawn(async move {
        while let Some(message) = reader.recv().await {
            match message {
                ServerToClientMessage::ConfigUpdated(config, _) => {
                    let config = match config.resolve() {
                        Ok(config) => config,
                        Err(error) => {
                            tracing::error!(?error, "failed to resolve the config");
                            continue;
                        }
                    };
                    app.emit("config-updated", config).unwrap();
                }
                ServerToClientMessage::OperationRequested(op, id) => app.emit("operation-requested", (op, id)).unwrap(),
                ServerToClientMessage::OperationPerformed(op, id, _) => app.emit("operation-performed", (op, id)).unwrap(),
                ServerToClientMessage::OperationFailed(op, id, error, _) => app.emit("operation-failed", (op, id, error.to_string())).unwrap(),
                ServerToClientMessage::ServerStateUpdated(state) => app.emit("server-state-updated", state).unwrap(),
                ServerToClientMessage::Stdout(buf) => app.emit("stdout", String::from_utf8_lossy(&buf)).unwrap(),
                ServerToClientMessage::Stderr(buf) => app.emit("stderr", String::from_utf8_lossy(&buf)).unwrap(),
                ServerToClientMessage::FatalError(error) => app.emit("fatal-error", error.to_string()).unwrap(),
                ServerToClientMessage::Error(error, _) => app.emit("error", error).unwrap(),
                ServerToClientMessage::ShuttingDown => app.emit("shutting-down", ()).unwrap(),
                _ => continue,
            }
        }
    });
}

fn browse_for_raphy_servers(
    app: &mut App<Wry>,
    servers: Arc<Mutex<IndexMap<String, Server>>>,
    runtime: &Runtime,
) -> anyhow::Result<()> {
    tracing::info!("create mdns service daemon");
    let service_daemon =
        mdns_sd::ServiceDaemon::new().context("Failed to create mDNS service daemon.")?;

    tracing::info!("browse for raphy servers");
    let receiver = service_daemon
        .browse(raphy_protocol::SERVICE_TYPE)
        .context("Failed to browse for the raphy servers.")?;

    let app_handle = app.handle().clone();

    runtime.spawn({
        async move {
            for event in receiver {
                let services_updated = match event {
                    ServiceEvent::ServiceResolved(info) => {
                        tracing::info!(?info, "server resolved");
                        servers.lock().await.insert(
                            info.get_fullname().to_owned(),
                            Server {
                                addresses: info.get_addresses().clone().into_iter().collect(),
                                port: info.get_port(),
                            },
                        );
                        true
                    }
                    ServiceEvent::ServiceRemoved(_, full_name) => {
                        tracing::info!(?full_name, "server removed");
                        // servers.lock().unwrap().shift_remove(&full_name);
                        true
                    }
                    _ => false,
                };

                if services_updated {
                    app_handle
                        .emit("servers-updated", servers.lock().await.clone())
                        .unwrap();
                }
            }
        }
    });

    Ok(())
}

fn real_setup(app: &mut App<Wry>, client_mode: ClientMode) -> anyhow::Result<()> {
    let servers = Arc::new(Mutex::new(IndexMap::new()));
    let runtime = Runtime::new().context("Failed to build the Tokio runtime.")?;

    let client = match client_mode {
        ClientMode::Remote => {
            browse_for_raphy_servers(app, Arc::clone(&servers), &runtime)?;
            None
        }
        ClientMode::Local => {
            let client = runtime
                .block_on(raphy_client::managed::from_unix(UNIX_SOCKET_PATH))
                .context("Failed to connect to the server.")?;
            Some(client)
        }
    };

    if let Some((reader, writer)) = &client {
        emit_message_on_s2c(&runtime, reader.clone(), app.handle().clone());
        emit_message_on_connection_failure(&runtime, writer.clone(), app.handle().clone())
    }

    app.manage(commands::AppState {
        servers,
        client: Mutex::new(client),
        runtime,
    });

    Ok(())
}

pub fn setup(client_mode: ClientMode) -> impl Fn(&mut App<Wry>) -> Result<(), Box<dyn Error>> {
    move |app| {
        let result = real_setup(app, client_mode);

        // the reason why we handle errors here is because `tauri` panics when the setup hook fails, so
        // if we handled it in the main function, this dialog would never be shown.
        //
        // additionally, on macOS systems at least a problem report window shows up, so we remind the
        // user that it will happen.
        //
        // this is just jank in general, we shouldn't need to do this
        if let Err(error) = result.as_ref() {
            #[cfg(debug_assertions)]
            let text_error = format!("{error:?}");

            #[cfg(not(debug_assertions))]
            let text_error = format!("{error:#}");

            if let Err(error) = native_dialog::MessageDialog::new()
                .set_title("raphy client application crashed.")
                .set_text(&format!("An error occurred during initialization.\n\n{text_error}\n\nThe program will now crash."))
                .set_type(MessageType::Error)
                .show_alert()
            {
                eprintln!("failed to show error dialog: {error}");
            }
        }

        result.map_err(Into::into)
    }
}
