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
    /// A core [`Message`] (i.e. a [`Launch`](libproto::system::Launch) message or a [`BeginMonitor`](libproto::system::BeginMonitor) message)
    /// was sent by a node without the required capability.
    IllegalCoreMessage {
        name: String,
        message: Message,
    },
    /// A test program has failed to send the initial `register` message, indicating that it might not be a test program at all
    MissingRegistration {
        name: String,
    },
    /// A non-test program has sent a `register` message, indicating that it might actually be a test program
    UnexpectedRegistration {
        name: String,
    },
    /// A core [`Message`] could not be handled, because it's type is unknown.
    UnknownCoreMessage {
        name: String,
        ty: String,
    },
    /// A node with an already existing name was attempted to be launched
    DuplicateNodeName {
        name: String,
    },
    /// A command name was not found in the list of registered commands
    UnknownCommand {
        command_name: String,
        available_commands: Vec<String>,
    },
    UnknownRunner {
        runner_name: String,
        available_runners: Vec<String>,
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
            CoreError::IllegalCoreMessage { name, message } => {
                write!(
                    f,
                    "node `{name}` tried to send a system message without the required capability: {}",
                    message.to_json()
                )
            }
            CoreError::MissingRegistration { name } => {
                write!(
                    f,
                    "node `{name}` has failed to send a registration message. Are you sure you have specified the correct command?"
                )
            }
            CoreError::UnexpectedRegistration { name } => {
                write!(
                    f,
                    "node `{name}` has attempted to send a registration message. Are you sure you have specified the correct command?"
                )
            }
            CoreError::UnknownCoreMessage { name, ty } => {
                write!(f, "unknown system message from node `{name}`: {ty}")
            }
            CoreError::DuplicateNodeName { name } => {
                write!(f, "a node with name `{name}` already exists")
            }
            CoreError::UnknownCommand { command_name,available_commands } => {
                write!(f, "a command with name `{command_name}` was not registered. Available commands are {available_commands:?}")
            }
            CoreError::UnknownRunner {runner_name, available_runners} => {
                write!(f, "a runner with name `{runner_name}`. Available runners are {available_runners:?}")
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
                        "source name does not match source id, expected `{:?}`, got `{:?}`",
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
