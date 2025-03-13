mod utils;

use anyhow::Context;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use directories::ProjectDirs;
use fs_err::tokio as fs;

pub const SERVICE_TYPE: &str = "_raphy._tcp.local.";
pub const INSTANCE_NAME: &str = "Raphy";

#[derive(Encode, Decode, Serialize, Deserialize, Clone)]
pub enum JavaPath {
    AutoDetect,
    Custom(PathBuf),
}

impl JavaPath {
    pub fn get(&self) -> Option<Cow<Path>> {
        match self {
            JavaPath::AutoDetect => utils::auto_detect_java_from_java_home_env()
                .or_else(utils::auto_detect_java_from_system_path)
                .map(Cow::Owned),
            JavaPath::Custom(path) => Some(Cow::Borrowed(path)),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Clone)]
pub enum ServerArguments {
    /// parse string using POSIX shell rules (`shlex`)
    Parsed(String),

    /// use the provided vector of strings as arguments
    Manual(Vec<String>),
}

impl ServerArguments {
    pub fn get(&self) -> anyhow::Result<Cow<[String]>> {
        match self {
            ServerArguments::Parsed(s) => Ok(Cow::Owned(shlex::split(s)
                .context("The provided server arguments contains erroneous input or syntax; please double check the arguments and try again.")?)),
            ServerArguments::Manual(args) => Ok(Cow::Borrowed(&args)),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Clone)]
pub enum User {
    /// launch the server as the current user
    Current,

    /// launch the server as the provided user
    Specific(String),
}

impl User {
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
}

#[derive(Encode, Decode, Serialize, Deserialize, Clone)]
pub struct Config {
    pub java_path: JavaPath,
    pub server_jar_path: PathBuf,
    pub arguments: ServerArguments,
    pub user: User,
}

impl Config {
    pub fn path() -> anyhow::Result<PathBuf> {
        match env::var_os("RAPHY_CONFIG_PATH") {
            Some(path) => Ok(PathBuf::from(path)),
            None => {
                match ProjectDirs::from("", "ALinuxPerson", "raphy") {
                    Some(pd) => Ok(pd.config_dir().join("config.json")),
                    None => Ok(env::current_dir().context("Failed to get the current directory.")?.join("config.json")),
                }
            }
        }
    }
    
    pub async fn load() -> anyhow::Result<Option<Self>> {
        let path = Self::path().context("Failed to get the config path.")?;
        
        if !path.exists() {
            return Ok(None);
        }
        
        let contents = fs::read_to_string(path).await.context("Failed to read the config file.")?;
        Ok(Some(serde_json::from_str(&contents).context("Failed to parse the config file.")?))
    }
    
    pub async fn dump(&self) -> anyhow::Result<()> {
        let path = Self::path().context("Failed to get the config path.")?;
    
        if let Some(path) = path.parent() {
            if let Err(error) = fs::create_dir_all(path).await {
                tracing::error!("failed to create the config directory: {error}");
            }
        }
        
        let contents = serde_json::to_string(self).context("Failed to serialize the config.")?;
        fs::write(path, contents).await.context("Failed to write the config file.")?;
        
        Ok(())
    }
}

#[derive(Encode, Decode, Copy, Clone)]
pub enum Operation {
    Start,
    Stop,
    Restart,
}

#[derive(Encode, Decode)]
pub enum ClientToServerMessage {
    UpdateConfig(Config),
    PerformOperation(Operation),
    Input(Vec<u8>),
    
    /// operation can only be performed by a local client
    Shutdown,
}

#[derive(Encode, Decode, Clone)]
pub enum ServerToClientMessage {
    ConfigUpdated(Config),
    ConfigNeeded,
    OperationRequested(Operation),
    OperationPerformed(Operation),
    ServerCrashed,
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    FatalError(String),
    ShuttingDown,
    NotLocalClient,
}
