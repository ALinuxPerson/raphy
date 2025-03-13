use anyhow::Context;
use mdns_sd::{ServiceDaemon, ServiceInfo};

pub fn start_advertising(port: u16) -> anyhow::Result<()> {
    tracing::info!("create mdns service daemon");
    let mdns = ServiceDaemon::new().context("Failed to create mDNS service daemon.")?;
    let service_info = ServiceInfo::new(
        raphy_protocol::SERVICE_TYPE,
        raphy_protocol::INSTANCE_NAME,
        &format!(
            "{}.{}",
            raphy_protocol::INSTANCE_NAME,
            raphy_protocol::SERVICE_TYPE
        ),
        "",
        port,
        None,
    )
    .expect("service info was invalid")
    .enable_addr_auto();

    tracing::info!("register service info with mdns");
    mdns.register(service_info)
        .context("Failed to register service info with mDNS.")?;

    Ok(())
}
