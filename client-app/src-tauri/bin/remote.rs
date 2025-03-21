// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use raphy_client::ClientMode;
use std::process::ExitCode;

fn main() -> ExitCode {
    raphy_client_app_lib::main(ClientMode::Remote)
}
