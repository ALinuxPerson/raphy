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

#[cfg(feature = "init_logging")]
pub use init_logging::init_logging;