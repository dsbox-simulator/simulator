//! System messages, that the core uses to communicate with its clients.
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::Message;
use crate::Payload;

/// Sent from the client process to the core when it wants to reset the simulation
/// all nodes will be shut down and then a [`ResetFinished`] message will be sent to the client
#[derive(Default, Payload, Serialize, Deserialize)]
pub struct Reset {}

/// Reply to a [`Reset`] message from the core, after initialization completes successfully.
#[derive(Payload, Serialize, Deserialize)]
pub struct ResetFinished {}

/// Sent from the client process to the core when it wants to launch a new server.
/// The `middleware_before` and `middleware_after` fields
/// are commands to be launched as the "middleware-stack" for the server
#[derive(Payload, Serialize, Deserialize)]
pub struct Launch {
    pub name: String,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub as_client: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub middleware_before: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub middleware_after: Vec<String>,
}


/// Reply to a [`Launch`] message from the core, after the server successfully launched.
#[derive(Payload, Serialize, Deserialize)]
pub struct LaunchFinished {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
}

/// Sent from a client node to the core to start a monitoring session. This can be used by client
/// nodes to monitor message exchange between nodes whose names match the given regexes.
#[derive(Payload, Serialize, Deserialize)]
pub struct BeginMonitor {
    pub src_match: String,
    pub dst_match: String,
}

/// Sent from the core to a client node that started a monitor session, whenever a message is sent
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


/// Sent from any node to the core in interactive mode to stop delivery of messages (to be resumed
/// by the user in the webapp)
#[derive(Payload, Serialize, Deserialize)]
pub struct Break {}