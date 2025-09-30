//! Errors that can happen during the execution of the simulation.
//!
//! Each [`CoreError`] variant describes an error that cannot be recovered from. Consequently the
//! currently running [`Core`](crate::core::Core) stops.

use std::fmt::{Display, Formatter};

use libproto::Message;
use crate::core::node::MiddlewareId;

/// An error that occurred during execution
pub enum CoreError {
    /// A [`Message`] could not be dispatched (either sent into the network, or delivered). The reason is given by [`DispatchErrorKind`].
    DispatchError {
        /// the name of the node that sent the message
        name: String,
        /// the [`Message`]s that was not dispatched.
        message: Message,
        /// the specific error that prevented dispatching.
        kind: DispatchErrorKind,
    },
    /// A core [`Message`] (i.e. a [`Launch`](libproto::system::Launch) message or a [`BeginMonitor`](libproto::system::BeginMonitor) message) was sent by a non-test node.
    IllegalCoreMessage { source: String, message: Message },
    /// A core [`Message`] could not be handled, because it's type is unknown.
    UnknownCoreMessage { source: String, ty: String },
    /// A message could not be forwarded to the next middleware, because the source process is last in the stack
    MissingMiddleware {
        /// the command that caused the error
        source: String,
        /// the name of the node in which the error occurred
        node: String,
        /// middleware index which tried to forward the message
        middleware_id: MiddlewareId,
    },
    /// An error occurred trying to launch a process.
    LaunchFailed { command: String, error: std::io::Error },
    /// A process wrote some text to its standard output, that could not be parsed into a [`Message`].
    SerializeError { source: String, raw_message: String, error: String },
}

/// Gives a reason why a [`Message`] could not be dispatched
pub enum DispatchErrorKind {
    /// The source name of a [`Message`] does not match the processes associated node name (or names, in case of the test process).
    SourceNameMismatch,
    /// The destination of a [`Message`] could not be resolved (the node name does not exist).
    DestinationUnknown,
}

impl Display for CoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::DispatchError { name, message, kind, .. } => {
                write!(f, "node `{name}` - dispatch error: {kind}; message: {}", message.to_json())
            }
            CoreError::IllegalCoreMessage { source, message } => {
                write!(f, "non-test process `{source}` tried to send core message: {}", message.to_json())
            }
            CoreError::UnknownCoreMessage { source, ty } => {
                write!(f, "unknown system message from `{source}`: {ty}")
            }
            CoreError::LaunchFailed { command, error: err } => {
                write!(f, "failed to launch process with command {command:?}: {err}")
            }
            CoreError::SerializeError { source, raw_message, error } => {
                write!(f, "failed to deserialize message from process `{source}`: {error} (raw message: {raw_message:?})")
            }
            CoreError::MissingMiddleware { source, node, middleware_id: middleware_idx } => {
                write!(f, "failed to forward message to next middleware process: `{node}` only has `{middleware_idx}` middleware(s). Process: `{source}`")
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