pub mod config;
mod error;
mod utils;

use bincode::{Decode, Encode};
pub use config::Config;
pub use error::SerdeError;
use serde::{Deserialize, Serialize};

pub const SERVICE_TYPE: &str = "_raphy._tcp.local.";
pub const INSTANCE_NAME: &str = "Raphy";
pub const UNIX_SOCKET_PATH: &str = "/tmp/raphy.sock";
pub const DEFAULT_PORT: u16 = 18000;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Copy, Clone)]
pub enum Operation {
    Start,
    Stop,
    Restart,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Copy, Clone)]
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

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Encode, Decode, Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct TaskId(Id);

impl TaskId {
    pub fn generate() -> Self {
        Self(Id::generate())
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct OperationId(Id);

impl OperationId {
    pub fn generate() -> Self {
        Self(Id::generate())
    }
}

#[derive(Debug, Encode, Decode)]
pub enum ClientToServerMessage {
    Ping(TaskId),
    GetConfig(TaskId),
    GetServerState(TaskId),
    UpdateConfig(TaskId, Config),
    PerformOperation(TaskId, Operation),
    Input(Vec<u8>),

    /// operation can only be performed by a local client
    Shutdown,
}

impl ClientToServerMessage {
    pub fn task_id(&self) -> Option<TaskId> {
        match self {
            Self::GetConfig(task_id)
            | Self::GetServerState(task_id)
            | Self::UpdateConfig(task_id, _)
            | Self::PerformOperation(task_id, _) => Some(*task_id),
            _ => None,
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ServerState {
    Started,
    Stopped(Option<ExitStatus>),
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum ServerToClientMessage {
    Pong(TaskId),
    CurrentConfig(Option<Config>, TaskId),
    CurrentServerState(ServerState, TaskId),
    ConfigUpdated(Config, Option<TaskId>),
    OperationRequested(Operation, OperationId),
    OperationPerformed(Operation, OperationId, Option<TaskId>),
    OperationFailed(Operation, OperationId, SerdeError, Option<TaskId>),
    ServerStateUpdated(ServerState),
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    FatalError(SerdeError),
    Error(SerdeError, Option<TaskId>),
    ShuttingDown,
}

impl ServerToClientMessage {
    pub fn task_id(&self) -> Option<TaskId> {
        match self {
            Self::Pong(task_id)
            | Self::CurrentConfig(_, task_id)
            | Self::CurrentServerState(_, task_id) => Some(*task_id),
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
