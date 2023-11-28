use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use libproto::Message;

pub enum CoreError {
    DispatchError {
        source: PathBuf,
        message: Message,
        kind: DispatchErrorKind,
    },
    IllegalCoreMessage(PathBuf, Message),
    UnknownSystemMessage(String),
    SpawnFailed(PathBuf, std::io::Error),
    SerializeError(PathBuf, String, serde_json::Error),
}

pub enum DispatchErrorKind {
    SourceNameMismatch,
    DestinationUnknown,
}

impl Display for CoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::DispatchError { message, kind, .. } => {
                write!(f, "dispatch error: {kind}; message: {}", message.to_json())
            }
            CoreError::IllegalCoreMessage(_, message) => {
                write!(f, "non-client node tried to send core message: {}", message.to_json())
            }
            CoreError::UnknownSystemMessage(ty) => {
                write!(f, "unknown system message: {ty}")
            }
            CoreError::SpawnFailed(path, err) => {
                write!(f, "failed to spawn process {}: {err}", path.display())
            }
            CoreError::SerializeError(path, raw_message, err) => {
                write!(f, "failed to deserialize message from process {}: {err} (raw message: {raw_message:?})", path.display())
            }
        }
    }
}

impl Display for DispatchErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchErrorKind::SourceNameMismatch => f.write_str("source name does not match source id"),
            DispatchErrorKind::DestinationUnknown => f.write_str("destination unknown"),
        }
    }
}