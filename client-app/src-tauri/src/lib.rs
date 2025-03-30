mod commands;
mod setup;
mod config;
pub mod utils;

pub use config::Config;

use raphy_client::{managed, ClientMode};
use setup::setup;
use std::env;
use tokio::runtime::Runtime;

pub fn run(client_mode: ClientMode, data: Option<(managed::ClientReader, managed::ClientWriter, Runtime)>) -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::connect_to_server,
            commands::client_connection_active,
            commands::client_mode,
            commands::get_server_config,
            commands::update_config,
            commands::start_server,
            commands::stop_server,
            commands::restart_server,
            commands::get_server_state,
        ])
        .register_asynchronous_uri_scheme_protocol("stdin", commands::stdin)
        .manage(client_mode)
        .setup(setup(client_mode, data))
        .run(tauri::generate_context!())
}
