use crate::setup;
use anyhow::Context;
use indexmap::{IndexMap, IndexSet};
use raphy_client::managed::{ClientReader, ClientWriter};
use raphy_client::ClientMode;
use raphy_protocol::config::resolved::{ConfigMask, ResolvedConfig};
use raphy_protocol::{Config, Operation};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tauri::{
    http, App, AppHandle, Emitter, Manager, State, UriSchemeContext, UriSchemeResponder, Wry,
};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use raphy_common::ConfigLike;

pub struct AppState {
    pub servers: Arc<Mutex<IndexMap<String, Server>>>,
    pub client: Mutex<Option<(ClientReader, ClientWriter)>>,
    pub runtime: Runtime,
    pub config: Mutex<crate::Config>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub addresses: IndexSet<IpAddr>,
    pub port: u16,
}

impl Server {
    pub fn socket_addresses(&self) -> impl Iterator<Item = SocketAddr> + '_ {
        self.addresses
            .iter()
            .map(move |address| SocketAddr::new(*address, self.port))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ConnectToServerBy {
    FullName(String),
    SocketAddress(SocketAddr),
}

#[tauri::command]
pub async fn connect_to_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    by: ConnectToServerBy,
) -> anyhow_tauri::TAResult<()> {
    tracing::info!(?by, "connect to server");

    tracing::debug!("lock servers structure");
    let servers = state.servers.lock().await;

    let socket_addresses = match by {
        ConnectToServerBy::FullName(full_name) => {
            let server = servers
                .get(&full_name)
                .context("The specified server does not exist.")?;
            server.socket_addresses().collect()
        }
        ConnectToServerBy::SocketAddress(socket_address) => vec![socket_address],
    };
    tracing::debug!(?socket_addresses, "evaluated socket addresses");

    tracing::debug!("connect to server");
    let client = tokio::time::timeout(
        Duration::from_secs(30),
        raphy_client::managed::from_tcp(socket_addresses.as_slice()),
    )
    .await
    .context("Connection timed out after 30 seconds.")?
    .context("Failed to connect to the server.")?;

    let client_reader = client.0.clone();
    let client_writer = client.1.clone();

    tracing::debug!("lock client structure and replace with new client");
    state.client.lock().await.replace(client);

    setup::emit_message_on_s2c(&state.runtime, client_reader, app_handle.clone());
    setup::emit_message_on_connection_failure(&state.runtime, client_writer, app_handle);

    tracing::info!("connected to server");
   
    let mut config = state.config.lock().await;
    config.last_remote_client = Some(socket_addresses);
    
    if let Err(error) = config.dump().await {
        tracing::warn!(?error, "failed to save the config: {error:#}");
    }
    
    drop(config);
   
    Ok(())
}

#[tauri::command]
pub async fn client_connection_active(
    state: State<'_, AppState>,
) -> anyhow_tauri::TAResult<bool> {
    tracing::debug!("lock client structure");
    let client = state.client.lock().await;
    let is_connected = client.is_some();
    tracing::debug!(?is_connected, "client connection active");
    Ok(is_connected)
}

#[tauri::command]
pub fn client_mode(state: State<'_, ClientMode>) -> ClientMode {
    *state
}

#[tauri::command]
pub async fn get_server_config(
    state: State<'_, AppState>,
) -> anyhow_tauri::TAResult<Option<(ResolvedConfig, ConfigMask)>> {
    tracing::debug!("lock client structure");
    let client = state.client.lock().await;
    let client_writer = client
        .as_ref()
        .context("Not connected to a server.")?
        .1
        .clone();
    drop(client);

    tracing::debug!("get server config");
    let config = client_writer
        .get_config()
        .await
        .context("Failed to get the server config.")?
        .map(|c| c.resolve().context("Failed to resolve the server config."))
        .transpose()?;

    tracing::debug!("server config retrieved");

    Ok(config)
}

#[tauri::command]
pub async fn update_config(
    state: State<'_, AppState>,
    config: ResolvedConfig,
    mask: ConfigMask,
) -> anyhow_tauri::TAResult<()> {
    let client = state.client.lock().await;
    let client_writer = client
        .as_ref()
        .context("Not connected to a server.")?
        .1
        .clone();
    drop(client);

    client_writer
        .update_config(Config::from_resolved(config, mask))
        .await
        .context("Failed to update the configuration.")?;
    Ok(())
}

async fn perform_operation(
    state: State<'_, AppState>,
    operation: Operation,
    op_done: &'static str,
) -> anyhow_tauri::TAResult<()> {
    tracing::debug!(?operation, ?op_done);

    tracing::debug!("lock client structure");
    let client = state.client.lock().await;
    let client_writer = client
        .as_ref()
        .context("Not connected to a server.")?
        .1
        .clone();
    drop(client);

    tracing::debug!("client writer perform operation");
    client_writer
        .perform_operation(operation)
        .await
        .with_context(|| format!("Failed to {op_done} the server."))?;
    Ok(())
}

#[tauri::command]
pub async fn start_server(state: State<'_, AppState>) -> anyhow_tauri::TAResult<()> {
    perform_operation(state, Operation::Start, "start").await
}

#[tauri::command]
pub async fn stop_server(state: State<'_, AppState>) -> anyhow_tauri::TAResult<()> {
    perform_operation(state, Operation::Stop, "stop").await
}

#[tauri::command]
pub async fn restart_server(state: State<'_, AppState>) -> anyhow_tauri::TAResult<()> {
    perform_operation(state, Operation::Restart, "restart").await
}

#[tauri::command]
pub async fn get_server_state(
    state: State<'_, AppState>,
) -> anyhow_tauri::TAResult<raphy_protocol::ServerState> {
    tracing::debug!("lock client structure");
    let client = state.client.lock().await;
    let client_writer = client
        .as_ref()
        .context("Not connected to a server.")?
        .1
        .clone();
    drop(client);

    tracing::debug!("get server state");
    let server_state = client_writer
        .get_server_state()
        .await
        .context("Failed to get the server state.")?;

    tracing::debug!("server state retrieved");

    Ok(server_state)
}

async fn real_stdin(state: &AppState, input: Vec<u8>) -> anyhow::Result<()> {
    let client = state.client.lock().await;
    let client_writer = client
        .as_ref()
        .context("Not connected to a server.")?
        .1
        .clone();
    drop(client);

    client_writer
        .input(input)
        .await
        .context("Failed to send input to the server.")?;
    Ok(())
}

pub fn stdin(
    ctx: UriSchemeContext<'_, Wry>,
    request: http::Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let body = request.into_body();

    ctx.app_handle().state::<AppState>().runtime.spawn({
        let app_handle = ctx.app_handle().clone();

        async move {
            let status = match real_stdin(&app_handle.state::<AppState>(), body).await {
                Ok(()) => 200,
                Err(error) => {
                    tracing::error!(?error, "failed to write to stdin: {error:#}");
                    500
                }
            };

            responder.respond(
                http::Response::builder()
                    .status(status)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Cow::Borrowed(&[][..]))
                    .unwrap(),
            )
        }
    });
}
