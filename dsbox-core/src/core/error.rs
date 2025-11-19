//! Errors that can happen during the execution of the simulation.
//!
//! Each [`CoreError`] variant describes an error that cannot be recovered from. Consequently the
//! currently running [`Core`](crate::core::Core) stops.

use std::fmt::{Display, Formatter};

use libproto::Message;

/// An error that occurred during execution
#[derive(Debug)]
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
    /// A test program has failed to send the initial `register` message, indicating that it might not be a test program at all
    MissingRegistration { source: String },
    /// A non-test program has sent a `register` message, indicating that it might actually be a test program
    UnexpectedRegistration { source: String },
    /// A core [`Message`] could not be handled, because it's type is unknown.
    UnknownCoreMessage { source: String, ty: String },
    /// An error occurred trying to launch a process.
    LaunchFailed {
        command: String,
        error: std::io::Error,
    },
    /// A process wrote some text to its standard output, that could not be parsed into a [`Message`].
    SerializeError {
        source: String,
        raw_message: String,
        error: String,
    },
}

/// Gives a reason why a [`Message`] could not be dispatched
#[derive(Debug)]
pub enum DispatchErrorKind {
    /// The source name of a [`Message`] does not match the processes associated node name (or names, in case of the test process).
    /// Contains the name that was given in the message and the name(s) of the node that sent the message (the test node might have several aliases)
    SourceNameMismatch(String, Vec<String>),
    /// The destination of a [`Message`] could not be resolved (the node name does not exist).
    DestinationUnknown,
}

impl Display for CoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::DispatchError {
                name,
                message,
                kind,
                ..
            } => {
                write!(
                    f,
                    "node `{name}` - dispatch error: {kind}; message: {}",
                    message.to_json()
                )
            }
            CoreError::IllegalCoreMessage { source, message } => {
                write!(
                    f,
                    "non-test process `{source}` tried to send core message: {}",
                    message.to_json()
                )
            }
            CoreError::MissingRegistration { source } => {
                write!(
                    f,
                    "process `{source}` has failed to register as a test. Are you sure you have specified the correct test program?"
                )
            }
            CoreError::UnexpectedRegistration { source } => {
                write!(
                    f,
                    "process `{source}` has attempted to register as a test. Are you sure you have specified the correct server program?"
                )
            }
            CoreError::UnknownCoreMessage { source, ty } => {
                write!(f, "unknown system message from `{source}`: {ty}")
            }
            CoreError::LaunchFailed {
                command,
                error: err,
            } => {
                write!(
                    f,
                    "failed to launch process with command {command:?}: {err}"
                )
            }
            CoreError::SerializeError {
                source,
                raw_message,
                error,
            } => {
                write!(
                    f,
                    "failed to deserialize message from process `{source}`: {error} (raw message: {raw_message:?})"
                )
            }
        }
    }
}

impl Display for DispatchErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchErrorKind::SourceNameMismatch(got, expected) => {
                if expected.len() == 1 {
                    write!(
                        f,
                        "source name does not match source id, expected one `{:?}`, got `{:?}`",
                        expected[0], got
                    )
                } else {
                    write!(
                        f,
                        "source name does not match source id, expected one of `{:?}`, got `{:?}`",
                        expected, got
                    )
                }
            }
            DispatchErrorKind::DestinationUnknown => f.write_str("destination unknown"),
        }
    }
}
