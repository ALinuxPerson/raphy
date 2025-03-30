mod base;
mod child;
mod network;
mod utils;

use crate::child::ChildTask;
use anyhow::Context;
use native_dialog::MessageType;
use raphy_protocol::Config;
use std::env;
use std::fmt::{Debug, Display};
use std::process::ExitCode;
use std::time::Duration;
use auto_launch::AutoLaunch;
use tokio::sync::mpsc;
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};
use tracing_subscriber::{EnvFilter, Layer};
use raphy_common::ConfigLike;

fn auto_launch() -> anyhow::Result<()> {
    let current_exe = env::current_exe().context("failed to get the current executable path")?;
    let current_exe = current_exe.to_str().context("failed to convert path to string")?;
    let auto_launch = AutoLaunch::new("raphy-server", current_exe, true, &[] as &[&str]);
    
    if auto_launch.is_enabled().context("Failed to check if auto-launch is enabled.")? {
        auto_launch.disable().context("Failed to disable auto-launch.")?;
        tracing::info!("auto-launch disabled");
    } else {
        auto_launch.enable().context("Failed to enable auto-launch.")?;   
        tracing::info!("auto-launch enabled");
    }
    
    Ok(())
}


async fn real_main(sh: SubsystemHandle<anyhow::Error>) -> anyhow::Result<()> {
    if env::args().nth(2).as_deref() == Some("auto-launch") {
        if let Err(error) = auto_launch() {
            tracing::warn!(?error, "failed to toggle auto-launch");
        }
    }
    
    let (n2s_tx, n2s_rx) = mpsc::unbounded_channel();
    let (global_s2c_tx, global_s2c_rx) = mpsc::unbounded_channel();
    let port = network::initialize(&sh, n2s_tx, global_s2c_rx)
        .await
        .context("Failed to initialize the network subsystem.")?;

    utils::start_advertising(port).context("Failed to start advertising mDNS service.")?;

    let config = Config::load()
        .await
        .context("Failed to load the server configuration.")?;
    let (s2ch_tx, s2ch_rx) = mpsc::unbounded_channel();
    let (ch2s_tx, ch2s_rx) = mpsc::unbounded_channel();
    let child_task = ChildTask::new(s2ch_rx, ch2s_tx, config.clone());

    sh.start(SubsystemBuilder::new("child", move |sh| async move {
        child_task.run(sh).await;
        Ok::<_, anyhow::Error>(())
    }));

    let server_task = base::ServerTask::new(n2s_rx, ch2s_rx, s2ch_tx, global_s2c_tx, config);
    sh.start(SubsystemBuilder::new("server", move |sh| async move {
        server_task.run(sh).await;
        Ok::<_, anyhow::Error>(())
    }));

    sh.on_shutdown_requested().await;
    Ok(())
}

async fn handle_error(error: impl Display + Debug + Send + Sync + 'static) {
    tracing::error!(?error, "{error:#}");

    tokio::task::spawn_blocking(move || {
        if let Err(error) = native_dialog::MessageDialog::new()
            .set_title("raphy server application crashed.")
            .set_text(&format!("One or more errors occurred.\n\n{error:?}"))
            .set_type(MessageType::Error)
            .show_alert()
        {
            tracing::error!("failed to show error dialog: {error}");
        }
    })
    .await
    .unwrap()
}

#[tokio::main]
async fn main() -> ExitCode {
    raphy_common::init_logging("RAPHY_SERVER_TOKIO_CONSOLE_ENABLED");

    if let Err(error) = Toplevel::new(|sh| async move {
        if let Err(error) = real_main(sh).await {
            handle_error(error).await
        }
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_secs(60))
    .await
    {
        handle_error(error).await;
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
