mod commands;

use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use anyhow::Context;
use mdns_sd::ServiceEvent;
use native_dialog::MessageType;
use tauri::{App, Manager, Wry};

fn real_setup(app: &mut App<Wry>) -> anyhow::Result<()> {
    let service_daemon = mdns_sd::ServiceDaemon::new().context("Failed to create mDNS service daemon.")?;
    let receiver = service_daemon.browse(raphy_protocol::SERVICE_TYPE).context("Failed to browse for the raphy mDNS service.")?;
    let services = Arc::new(Mutex::new(Vec::new()));
    
    thread::spawn({
        let services = Arc::clone(&services);
        
        move || {
            for event in receiver {
                println!("{event:#?}")
                // match event {
                //     ServiceEvent::ServiceFound(_, _) => {}
                //     ServiceEvent::ServiceResolved(_) => {}
                //     ServiceEvent::ServiceRemoved(_, _) => {}
                //     ServiceEvent::SearchStopped(_) => {}
                // }
            }
        }
    });
    
    app.manage(commands::AppState {
        service_daemon,
        services
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
        .invoke_handler(tauri::generate_handler![commands::discover_servers])
        .setup(setup)
        .run(tauri::generate_context!())
}
