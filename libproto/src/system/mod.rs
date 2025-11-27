//! System messages, that the core uses to communicate with nodes.

pub mod control;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::Message;
use crate::Payload;

/// Sent from a node to the core when starting. Only used to implement better error messages
/// in case the end user mixes up commands
#[derive(Default, Payload, Serialize, Deserialize)]
pub struct Register {}

/// Sent from a node to the core when it wants to reset the simulation
/// all nodes launched by it will be shut down and then a [`ResetFinished`] reply is sent
#[derive(Default, Payload, Serialize, Deserialize)]
pub struct Reset {}

/// Reply to a [`Reset`] message from the core, after initialization completes successfully.
#[derive(Payload, Serialize, Deserialize)]
pub struct ResetFinished {}

/// Sent from a node to the core when it wants to launch a new node.
#[derive(Payload, Serialize, Deserialize)]
pub struct Launch {
    /// the name of the node
    pub name: String,

    /// the name of the command to launch. Commands must be registered with the core before they can be used
    pub command_name: String,

    /// if set, the core will send an `exited` message to the test when this node's process terminates.
    #[serde(skip_serializing_if = "std::ops::Not::not", default = "true_if_missing")]
    pub request_exited_message: bool,
}

/// Sent from a node to the core when it wants to register a new alias for itself.
#[derive(Payload, Serialize, Deserialize)]
pub struct Alias {
    /// the name of the alias
    pub name: String
}

/// Sent from the core to a node when another node launched by it has exited (if requested in [`Launch`])
#[derive(Payload, Serialize, Deserialize)]
pub struct Exited {
    pub name: String,
    pub exit_code: i32,
}

/// Reply to a [`Launch`] message from the core, after the node successfully launched.
#[derive(Payload, Serialize, Deserialize)]
pub struct LaunchFinished {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
}

/// Sent from a node to the core to start a monitoring session. This can be used to monitor
/// message exchange between nodes whose names match the given regexes.
#[derive(Payload, Serialize, Deserialize)]
pub struct BeginMonitor {
    pub src_match: String,
    pub dst_match: String,
}

/// Sent from the core to a test node that started a monitor session, whenever a message is sent
/// or delivered from or to a matching node.
#[derive(Payload, Serialize, Deserialize)]
pub struct MonitorEvent {
    pub kind: MonitorEventKind,
    pub timestamp_logical: usize,
    pub timestamp_physical: DateTime<Local>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reference_to: Option<usize>,

    pub message: Message,
}

#[derive(Serialize, Deserialize)]
pub enum MonitorEventKind {
    Sent,
    Delivered,
}

fn true_if_missing() -> bool {
    true
}
