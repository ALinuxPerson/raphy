use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use indexmap::{IndexMap, IndexSet};
use mdns_sd::ServiceDaemon;
use serde::{Deserialize, Serialize};

pub struct AppState {
    pub service_daemon: ServiceDaemon,
    pub services: Arc<Mutex<IndexMap<String, Server>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub addresses: IndexSet<IpAddr>,
    pub port: u16,
}

#[tauri::command]
pub async fn connect_to_server(full_name: String) -> Result<(), String> {
    println!("{full_name}");
    Err("test wee woo".to_owned())
}

