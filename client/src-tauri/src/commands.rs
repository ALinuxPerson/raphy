use std::sync::{Arc, Mutex};
use mdns_sd::ServiceDaemon;
use serde::{Deserialize, Serialize};
use tauri::State;

pub struct AppState {
    pub service_daemon: ServiceDaemon,
    pub services: Arc<Mutex<Vec<Server>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    id: String,
    hostname: String,
    ip: String,
}

#[tauri::command]
pub async fn discover_servers(state: State<'_, AppState>) -> Result<(), String> {
    // state.service_daemon.browse(raphy_protocol::SERVICE_TYPE)
    todo!()
}
