pub mod resolved {
    use crate::Config;
    use crate::config::{JavaPath, JavaPathKind, Arguments, User, UserKind};
    use anyhow::Context;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    #[derive(Serialize, Deserialize, Clone)]
    pub struct ResolvedConfig {
        pub java_path: PathBuf,
        pub server_jar_path: PathBuf,
        pub java_arguments: Arguments,
        pub server_arguments: Arguments,
        pub user: Option<String>,
    }

    #[derive(Serialize, Deserialize, Copy, Clone)]
    pub struct ConfigMask {
        pub java_path: JavaPathKind,
        pub user: UserKind,
    }

    impl Config {
        pub fn resolve(&self) -> anyhow::Result<(ResolvedConfig, ConfigMask)> {
            Ok((
                ResolvedConfig {
                    java_path: self
                        .java_path
                        .resolve()
                        .map(|jp| jp.to_path_buf())
                        .context(
                            "Failed to get the Java path. Is Java installed in your system?",
                        )?,
                    server_jar_path: self.server_jar_path.clone(),
                    server_arguments: self.server_arguments.clone(),
                    java_arguments: self.java_arguments.clone(),
                    user: self.user.resolve().map(|u| u.to_owned()),
                },
                ConfigMask {
                    java_path: self.java_path.kind(),
                    user: self.user.kind(),
                },
            ))
        }

        pub fn from_resolved(config: ResolvedConfig, mask: ConfigMask) -> Self {
            Self {
                java_path: match mask.java_path {
                    JavaPathKind::AutoDetect => JavaPath::AutoDetect,
                    JavaPathKind::Custom => JavaPath::Custom(config.java_path),
                },
                server_jar_path: config.server_jar_path,
                server_arguments: config.server_arguments,
                java_arguments: config.java_arguments,
                user: match (config.user, mask.user) {
                    (Some(user), UserKind::Specific) => User::Specific(user),
                    (_, UserKind::Current) => User::Current,
                    _ => panic!("invalid user configuration"),
                },
            }
        }
    }
}

use crate::utils;
use anyhow::Context;
use bincode::{Decode, Encode};
use directories::ProjectDirs;
use fs_err::tokio as fs;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum JavaPathKind {
    AutoDetect,
    Custom,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone)]
pub enum JavaPath {
    AutoDetect,
    Custom(PathBuf),
}

impl JavaPath {
    pub fn resolve(&self) -> Option<Cow<Path>> {
        match self {
            Self::AutoDetect => utils::auto_detect_java_from_java_home_env()
                .or_else(utils::auto_detect_java_from_system_path)
                .map(Cow::Owned),
            Self::Custom(path) => Some(Cow::Borrowed(path)),
        }
    }

    pub fn kind(&self) -> JavaPathKind {
        match self {
            Self::AutoDetect => JavaPathKind::AutoDetect,
            Self::Custom(_) => JavaPathKind::Custom,
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum ServerArgumentsKind {
    Parsed,
    Manual,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone)]
pub enum Arguments {
    /// parse string using POSIX shell rules (`shlex`)
    Parsed(String),

    /// use the provided vector of strings as arguments
    Manual(Vec<String>),
}

impl Arguments {
    pub fn resolve(&self) -> anyhow::Result<Cow<[String]>> {
        match self {
            Self::Parsed(s) => Ok(Cow::Owned(shlex::split(s)
                .context("The provided server arguments contains erroneous input or syntax; please double check the arguments and try again.")?)),
            Self::Manual(args) => Ok(Cow::Borrowed(args)),
        }
    }

    pub fn kind(&self) -> ServerArgumentsKind {
        match self {
            Self::Parsed(_) => ServerArgumentsKind::Parsed,
            Self::Manual(_) => ServerArgumentsKind::Manual,
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum UserKind {
    Current,
    Specific,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone)]
pub enum User {
    /// launch the server as the current user
    Current,

    /// launch the server as the provided user
    Specific(String),
}

impl User {
    pub fn resolve(&self) -> Option<&str> {
        match self {
            Self::Current => None,
            Self::Specific(user) => Some(user),
        }
    }

    pub fn make_command(&self) -> Option<Command> {
        match self {
            Self::Current => None,
            Self::Specific(user) => {
                let mut command = Command::new("sudo");
                command.args(["-u", user]);
                Some(command)
            }
        }
    }

    pub fn kind(&self) -> UserKind {
        match self {
            Self::Current => UserKind::Current,
            Self::Specific(_) => UserKind::Specific,
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub java_path: JavaPath,
    pub server_jar_path: PathBuf,
    pub java_arguments: Arguments,
    pub server_arguments: Arguments,
    pub user: User,
}

impl Config {
    pub fn path() -> anyhow::Result<PathBuf> {
        match env::var_os("RAPHY_CONFIG_PATH") {
            Some(path) => Ok(PathBuf::from(path)),
            None => match ProjectDirs::from("", "ALinuxPerson", "raphy") {
                Some(pd) => Ok(pd.config_dir().join("config.json")),
                None => Ok(env::current_dir()
                    .context("Failed to get the current directory.")?
                    .join("config.json")),
            },
        }
    }

    pub async fn load() -> anyhow::Result<Option<Self>> {
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

    pub async fn dump(&self) -> anyhow::Result<()> {
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
