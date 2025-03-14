mod commands;

use crate::commands::Server;
use anyhow::Context;
use indexmap::IndexMap;
use mdns_sd::ServiceEvent;
use native_dialog::MessageType;
use std::error::Error;
use std::sync::Arc;
use tauri::{App, Emitter, Manager, Wry};
use tokio::sync::Mutex;

fn real_setup(app: &mut App<Wry>) -> anyhow::Result<()> {
    let service_daemon =
        mdns_sd::ServiceDaemon::new().context("Failed to create mDNS service daemon.")?;
    let receiver = service_daemon
        .browse(raphy_protocol::SERVICE_TYPE)
        .context("Failed to browse for the raphy servers.")?;
    let servers = Arc::new(Mutex::new(IndexMap::new()));

    let runtime = tokio::runtime::Runtime::new().context("Failed to build the Tokio runtime.")?;
    let app_handle = app.handle().clone();
    runtime.spawn({
        let servers = Arc::clone(&servers);

        async move {
            for event in receiver {
                let services_updated = match event {
                    ServiceEvent::ServiceResolved(info) => {
                        println!("server resolved: {info:#?}");
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
                        println!("server removed: {full_name}");
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

    app.manage(commands::AppState {
        service_daemon,
        servers,
        client: Mutex::new(None),
        runtime,
    });

    Ok(())
}

fn setup(app: &mut App<Wry>) -> Result<(), Box<dyn Error>> {
    let result = real_setup(app);

    // the reason why we handle errors here is because `tauri` panics when the setup hook fails, so
    // if we handled it in the main function, this dialog would never be shown.
    //
    // additionally, on macOS systems at least a problem report window shows up, so we remind the
    // user that it will happen.
    //
    // this is just jank in general, we shouldn't need to do this
    if let Err(error) = result.as_ref() {
        if let Err(error) = native_dialog::MessageDialog::new()
            .set_title("raphy client application crashed.")
            .set_text(&format!("An error occurred during initialization.\n\n{error:?}\n\nThe program will now crash."))
            .set_type(MessageType::Error)
            .show_alert()
        {
            eprintln!("failed to show error dialog: {error}");
        }
    }

    result.map_err(Into::into)
}

pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::connect_to_server])
        .setup(setup)
        .run(tauri::generate_context!())
}
