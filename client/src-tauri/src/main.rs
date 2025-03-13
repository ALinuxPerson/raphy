// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;
use native_dialog::MessageType;

fn main() -> ExitCode {
    if let Err(error) = raphy_client_lib::run() {
        
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
