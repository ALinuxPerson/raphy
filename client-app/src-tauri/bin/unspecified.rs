// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use raphy_client::ClientMode;
use std::process::ExitCode;

fn main() -> ExitCode {
    let client_mode = match native_dialog::MessageDialog::new()
        .set_title("raphy client application needs additional input to continue")
        .set_text("The client needs additional input in order to continue.\n\nAre you launching as a local client?\nIf no, remote client will be assumed.")
        .show_confirm() {
        Ok(is_local_client) => if is_local_client {
            ClientMode::Local
        } else {
            ClientMode::Remote
        },
        Err(error) => {
            tracing::error!(?error, "failed to show confirm dialog: {error}");
            return ExitCode::FAILURE
        }
    };

    raphy_client_app_lib::main(client_mode)
}
