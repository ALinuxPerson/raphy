use anyhow::Context;
use raphy_client::managed;
use std::future::Future;
use std::time::Duration;
use tokio::io;
use tokio::net::ToSocketAddrs;
use raphy_protocol::UNIX_SOCKET_PATH;

pub async fn attempt_connection<F>(
    mut connect: impl FnMut() -> F,
    with_retry: bool,
) -> anyhow::Result<(managed::ClientReader, managed::ClientWriter)>
where
    F: Future<Output = io::Result<(managed::ClientReader, managed::ClientWriter)>>,
{
    let mut tries = if with_retry { 3 } else { 1 };
    let (client_reader, client_writer) = loop {
        match connect().await {
            Ok(client) => break client,
            Err(error) => {
                tries -= 1;
                tracing::debug!(?error, "failed to connect to server");
                if tries == 0 {
                    return Err(error).context("Failed to connect to the server.");
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    };

    tokio::time::timeout(Duration::from_secs(3), client_writer.ping())
        .await
        .context("Ping to server timed out after 3 seconds.")?
        .context("Failed to ping the server.")?;

    Ok((client_reader, client_writer))
}

pub async fn attempt_connection_via_unix(
    with_retry: bool,
) -> anyhow::Result<(managed::ClientReader, managed::ClientWriter)> {
    attempt_connection(
        || managed::from_unix(UNIX_SOCKET_PATH),
        with_retry,
    )
    .await
}

pub async fn attempt_connection_via_tcp(
    socket_addresses: impl ToSocketAddrs + Clone,
    with_retry: bool,
) -> anyhow::Result<(managed::ClientReader, managed::ClientWriter)> {
    attempt_connection(
        || managed::from_tcp(socket_addresses.clone()),
        with_retry,
    )
    .await
}
