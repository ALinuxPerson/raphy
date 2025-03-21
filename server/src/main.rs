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
use tokio::sync::mpsc;
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};
use tracing_subscriber::{EnvFilter, Layer};

async fn real_main(sh: SubsystemHandle<anyhow::Error>) -> anyhow::Result<()> {
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
