#[cfg(unix)]
mod client_mode {
    use std::{env, iter};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tokio::runtime::Runtime;
    use raphy_client::{managed, ClientMode};
    use raphy_client_app_lib::utils::attempt_connection_via_unix;
    use std::os::unix::process::CommandExt;

    fn find_raphy_server_bin_path() -> Option<PathBuf> {
        env::var_os("RAPHY_CLIENT_APP_SERVER_BIN_PATH")
            .map(PathBuf::from)
            .or_else(|| {
                for path in
                    env::split_paths(&env::var_os("PATH")?).chain(iter::once(PathBuf::from(".")))
                {
                    let raphy_server_bin_path = path.join("raphy-server");
                    if raphy_server_bin_path.exists() {
                        return Some(raphy_server_bin_path);
                    }
                }

                None
            })
    }

    fn spawn_detached(program: &Path) -> anyhow::Result<()> {
        let mut command = Command::new(program);

        unsafe {

            command
                .pre_exec(|| {
                    if let Err(error) = nix::unistd::setsid() {
                        tracing::error!("failed to run setsid: {error}");
                    }

                    Ok(())
                })
                .spawn()
                .context("failed to spawn the process")?;
        }

        Ok(())
    }

    fn real_infer_client_mode() -> anyhow::Result<(
        ClientMode,
        Option<(managed::ClientReader, managed::ClientWriter, Runtime)>,
    )> {
        let runtime = Runtime::new().context("Failed to build the Tokio runtime.")?;

        match runtime.block_on(attempt_connection_via_unix(false)) {
            Ok((cr, cw)) => return Ok((ClientMode::Local, Some((cr, cw, runtime)))),
            Err(error) => {
                tracing::debug!(
                ?error,
                "failed to connect to server via unix socket, attempting to start server instead"
            );
            }
        };

        match find_raphy_server_bin_path() {
            Some(path) => {
                if let Err(error) = spawn_detached(&path) {
                    tracing::error!(
                    ?error,
                    "failed to spawn the raphy server process: {error:#}"
                );
                    Ok((ClientMode::Remote, None))
                } else {
                    match runtime.block_on(attempt_connection_via_unix(true)) {
                        Ok((cr, cw)) => Ok((ClientMode::Local, Some((cr, cw, runtime)))),
                        Err(error) => {
                            tracing::warn!(
                            ?error,
                            "failed to connect to server via unix socket after spawning the server"
                        );
                            Ok((ClientMode::Remote, None))
                        }
                    }
                }
            }
            None => Ok((ClientMode::Remote, None)),
        }
    }

    fn infer_client_mode(
        data: &mut Option<(managed::ClientReader, managed::ClientWriter, Runtime)>,
    ) -> anyhow::Result<ClientMode> {
        real_infer_client_mode().map(|(mode, result)| {
            *data = result;
            mode
        })
    }

    pub fn client_mode(
        data: &mut Option<(managed::ClientReader, managed::ClientWriter, Runtime)>,
    ) -> anyhow::Result<ClientMode> {
        match env::var("RAPHY_CLIENT_APP_CLIENT_MODE") {
            Ok(mode) => match mode.as_str() {
                "local" => Ok(ClientMode::Local),
                "remote" => Ok(ClientMode::Remote),
                _ => infer_client_mode(data),
            },
            Err(_) => infer_client_mode(data),
        }
    }

}

use anyhow::Context;
use raphy_client::{managed, ClientMode};
use raphy_client_app_lib::Config;
use raphy_common::ConfigLike;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::{env, iter};
use tokio::runtime::Runtime;

fn main() -> ExitCode {
    raphy_common::init_logging("RAPHY_CLIENT_APP_TOKIO_CONSOLE_ENABLED");

    let mut data = None;
    
    #[cfg(unix)]
    let client_mode = match client_mode::client_mode(&mut data) {
        Ok(mode) => mode,
        Err(error) => {
            tracing::error!(?error, "failed to determine the client mode");
            return ExitCode::FAILURE;
        }
    };
    
    #[cfg(not(unix))]
    let client_mode = ClientMode::Remote;
    
    if let Err(error) = raphy_client_app_lib::run(client_mode, data) {
        tracing::error!(?error, "failed to run the client app: {error}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
