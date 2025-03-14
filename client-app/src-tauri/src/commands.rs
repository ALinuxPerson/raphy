use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use anyhow::Context;
use indexmap::{IndexMap, IndexSet};
use mdns_sd::ServiceDaemon;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use raphy_client::managed::{ClientReader, ClientWriter};

pub struct AppState {
    pub service_daemon: ServiceDaemon,
    pub servers: Arc<Mutex<IndexMap<String, Server>>>,
    pub client: Mutex<Option<(ClientReader, ClientWriter)>>,
    pub runtime: Runtime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub addresses: IndexSet<IpAddr>,
    pub port: u16,
}

impl Server {
    pub fn socket_addresses(&self) -> impl Iterator<Item = SocketAddr> + '_ {
        self.addresses.iter().map(move |address| SocketAddr::new(*address, self.port))
    }
}

#[tauri::command]
pub async fn connect_to_server(state: State<'_, AppState>, full_name: String) -> anyhow_tauri::TAResult<()> {
    let servers = state.servers.lock().await; 
    let server = servers.get(&full_name).context("The specified server does not exist.")?;
    let socket_addresses: Vec<_> = server.socket_addresses().collect();
    let client = raphy_client::managed::from_tcp(socket_addresses.as_slice()).await.context("Failed to connect to the server.")?;
    
    state.client.lock().await.replace(client);
    
    Ok(())
}

