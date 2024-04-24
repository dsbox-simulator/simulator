//! Errors that can happen during the execution of the simulation.
//!
//! Each [`CoreError`] variant describes an error that cannot be recovered from. Consequently the
//! currently running [`Core`](crate::core::Core) stops.

use std::fmt::{Display, Formatter};

use libproto::Message;

/// An error that occurred during execution
pub enum CoreError {
    /// A [`Message`] could not be dispatched (either sent into the network, or delivered). The reason is given by [`DispatchErrorKind`].
    DispatchError {
        /// the executable file that caused the error.
        source: String,
        /// the [`Message`]s that was not dispatched.
        message: Message,
        /// the specific error that prevented dispatching.
        kind: DispatchErrorKind,
    },
    /// A core [`Message`] (i.e. a [`Setup`](libproto::system::Setup) message or a [`BeginMonitor`](libproto::system::BeginMonitor) message) was sent by a non-client node.
    IllegalCoreMessage(String, Message),
    /// A core [`Message`] could not be handled, because it's type is unknown.
    UnknownCoreMessage(String),
    /// An error occurred trying to launch a process.
    LaunchFailed(String, std::io::Error),
    /// A process wrote some text to its standard output, that could not be parsed into a [`Message`].
    SerializeError(String, String, serde_json::Error),
}

/// Gives a reason why a [`Message`] could not be dispatched
pub enum DispatchErrorKind {
    /// The source name of a [`Message`] does not match the processes associated node name (or names, in case of the client process).
    SourceNameMismatch,
    /// The destination of a [`Message`] could not be resolved (the node name does not exist).
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
            CoreError::UnknownCoreMessage(ty) => {
                write!(f, "unknown system message: {ty}")
            }
            CoreError::LaunchFailed(command, err) => {
                write!(f, "failed to launch process with command {command:?}: {err}")
            }
            CoreError::SerializeError(path, raw_message, err) => {
                write!(f, "failed to deserialize message from process {}: {err} (raw message: {raw_message:?})", path)
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