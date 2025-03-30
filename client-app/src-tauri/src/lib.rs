mod commands;
mod setup;

use raphy_client::ClientMode;
use setup::setup;
use std::env;
use std::process::ExitCode;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub fn run(client_mode: ClientMode) -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::connect_to_server,
            commands::client_mode,
            commands::get_server_config,
            commands::update_config,
            commands::start_server,
            commands::stop_server,
            commands::restart_server,
        ])
        .register_asynchronous_uri_scheme_protocol("stdin", commands::stdin)
        .manage(client_mode)
        .setup(setup(client_mode))
        .run(tauri::generate_context!())
}

pub fn main(client_mode: ClientMode) -> ExitCode {
    raphy_common::init_logging("RAPHY_CLIENT_APP_TOKIO_CONSOLE_ENABLED");

    if let Err(_error) = run(client_mode) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
