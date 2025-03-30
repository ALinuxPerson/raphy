#[cfg(feature = "init_logging")]
mod init_logging {
    use std::env;
    use tracing_subscriber::{EnvFilter, Layer};
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    pub fn init_logging(tokio_console_var: &str) {
        let registry = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer().with_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
        );

        if env::var(tokio_console_var) == Ok("1".to_owned()) {
            registry.with(console_subscriber::spawn()).init();
            tracing::info!("tokio console is enabled");
        } else {
            registry.init();
        }
    }
}

#[cfg(feature = "config")]
mod config {
    use std::env;
    use std::path::PathBuf;
    use anyhow::Context;
    use directories::ProjectDirs;
    use serde::Serialize;
    use fs_err::tokio as fs;
    use serde::de::DeserializeOwned;

    #[allow(async_fn_in_trait)]
    pub trait ConfigLike: Serialize + DeserializeOwned {
        const ENV_VAR: &'static str;
        const CONFIG_PATH_NAME: &'static str;
        
        fn path() -> anyhow::Result<PathBuf> {
            match env::var_os(Self::ENV_VAR) {
                Some(path) => Ok(PathBuf::from(path)),
                None => match ProjectDirs::from("", "ALinuxPerson", "raphy") {
                    Some(pd) => Ok(pd.config_dir().join(Self::CONFIG_PATH_NAME)),
                    None => Ok(env::current_dir()
                        .context("Failed to get the current directory.")?
                        .join(Self::CONFIG_PATH_NAME)),
                },
            }
        }

        async fn load() -> anyhow::Result<Option<Self>> {
            let path = Self::path().context("Failed to get the config path.")?;

            if !path.exists() {
                return Ok(None);
            }

            let contents = fs::read_to_string(path)
                .await
                .context("Failed to read the config file.")?;
            Ok(Some(
                serde_json::from_str(&contents).context("Failed to parse the config file.")?,
            ))
        }

        async fn dump(&self) -> anyhow::Result<()> {
            let path = Self::path().context("Failed to get the config path.")?;

            if let Some(path) = path.parent() {
                if let Err(error) = fs::create_dir_all(path).await {
                    tracing::error!("failed to create the config directory: {error}");
                }
            }

            let contents = serde_json::to_string(self).context("Failed to serialize the config.")?;
            fs::write(path, contents)
                .await
                .context("Failed to write the config file.")?;

            Ok(())
        }
    }
}

#[cfg(feature = "init_logging")]
pub use init_logging::init_logging;

#[cfg(feature = "config")]
pub use config::ConfigLike;