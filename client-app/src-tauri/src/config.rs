use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use raphy_common::ConfigLike;

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub last_remote_client: Option<Vec<SocketAddr>>,
}

impl ConfigLike for Config {
    const ENV_VAR: &'static str = "RAPHY_CLIENT_APP_CONFIG_PATH";
    const CONFIG_PATH_NAME: &'static str = "client.json";
}