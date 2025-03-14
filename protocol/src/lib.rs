mod utils;
mod error {
    use bincode::{Decode, Encode};
    use serde::{Deserialize, Serialize};
    use std::fmt;

    #[derive(Clone, Deserialize, Serialize, Encode, Decode)]
    pub struct SerdeError {
        display: String,
        alt_display: String,
        debug: String,
        alt_debug: String,
        source: Option<Box<Self>>,
    }

    impl SerdeError {
        pub fn new<T>(e: &T) -> Self
        where
            T: ?Sized + std::error::Error,
        {
            Self {
                display: e.to_string(),
                alt_display: format!("{e:#}"),
                debug: format!("{e:?}"),
                alt_debug: format!("{e:#?}"),
                source: e.source().map(|s| Box::new(Self::new(s))),
            }
        }
    }

    impl std::error::Error for SerdeError {
        fn source(&self) -> Option<&(dyn 'static + std::error::Error)> {
            self.source
                .as_ref()
                .map(|s| &**s as &(dyn 'static + std::error::Error))
        }

        fn description(&self) -> &str {
            &self.display
        }
    }

    impl fmt::Display for SerdeError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if f.alternate() {
                write!(f, "{:#}", self.display)
            } else {
                write!(f, "{}", self.display)
            }
        }
    }

    impl fmt::Debug for SerdeError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if f.alternate() {
                write!(f, "{:#?}", self.display)
            } else {
                write!(f, "{:?}", self.display)
            }
        }
    }
}

pub use error::SerdeError;
use anyhow::Context;
use bincode::{Decode, Encode};
use directories::ProjectDirs;
use fs_err::tokio as fs;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};
use tokio::process::Command;

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

#[derive(Encode, Decode, Copy, Clone)]
pub enum Operation {
    Start,
    Stop,
    Restart,
}

#[derive(Encode, Decode, Copy, Clone)]
pub enum ExitStatus {
    Success,
    Failure,
}

impl From<std::process::ExitStatus> for ExitStatus {
    fn from(status: std::process::ExitStatus) -> Self {
        if status.success() {
            Self::Success
        } else {
            Self::Failure
        }
    }
}

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq)]
pub struct Id(u64);

impl Id {
    pub fn generate() -> Self {
        Self(rand::random())
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::generate()
    }
}

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, Default)]
pub struct TaskId(Id);

impl TaskId {
    pub fn generate() -> Self {
        Self(Id::generate())
    }
}

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, Default)]
pub struct OperationId(Id);

impl OperationId {
    pub fn generate() -> Self {
        Self(Id::generate())
    }
}

#[derive(Encode, Decode)]
pub enum ClientToServerMessage {
    UpdateConfig(TaskId, Config),
    PerformOperation(TaskId, Operation),
    Input(Vec<u8>),

    /// operation can only be performed by a local client
    Shutdown,
}

#[derive(Encode, Decode, Clone)]
pub enum ServerToClientMessage {
    ConfigUpdated(Config, Option<TaskId>),
    OperationRequested(Operation, OperationId),
    OperationPerformed(Operation, OperationId, Option<TaskId>),
    OperationFailed(Operation, OperationId, SerdeError, Option<TaskId>),
    ServerUnexpectedlyExited(Option<ExitStatus>),
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    FatalError(SerdeError),
    Error(SerdeError, Option<TaskId>),
    ShuttingDown,
}

impl ServerToClientMessage {
    pub fn task_id(&self) -> Option<TaskId> {
        match self {
            Self::ConfigUpdated(_, task_id)
            | Self::OperationPerformed(_, _, task_id)
            | Self::OperationFailed(_, _, _, task_id)
            | Self::Error(_, task_id) => *task_id,
            _ => None,
        }
    }

    pub fn operation_id(&self) -> Option<OperationId> {
        match self {
            Self::OperationRequested(_, operation_id)
            | Self::OperationPerformed(_, operation_id, _)
            | Self::OperationFailed(_, operation_id, _, _) => Some(*operation_id),
            _ => None,
        }
    }
}
