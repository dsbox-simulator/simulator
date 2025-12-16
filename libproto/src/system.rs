//! System messages, that the core uses to communicate with tests.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::Message;
use crate::Payload;

/// Sent from a test node to the core when starting. Only used to implement better error messages
/// in case someone mixes up test and server programs
#[derive(Default, Payload, Serialize, Deserialize)]
pub struct Register {}

/// Sent from the test process to the core when it wants to reset the simulation
/// all nodes will be shut down and then a [`ResetFinished`] message will be sent to the test
#[derive(Default, Payload, Serialize, Deserialize)]
pub struct Reset {}

/// Reply to a [`Reset`] message from the core, after initialization completes successfully.
#[derive(Payload, Serialize, Deserialize)]
pub struct ResetFinished {}

/// Sent from the test process to the core when it wants to launch a new server.
/// The `middleware_before` and `middleware_after` fields
/// are commands to be launched as the "middleware-stack" for the server
#[derive(Payload, Serialize, Deserialize)]
pub struct Launch {
    /// the name of the node
    pub name: String,
    /// whether this node is a test node (in which case no process is actually launched) or a
    /// regular node, that will be launched as an independent process
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub as_test: bool,
    /// if set, the core will send an `exited` message to the test when this node's process terminates.
    /// Only works if `as_test` is `false`.
    #[serde(skip_serializing_if = "std::ops::Not::not", default = "true_if_missing")]
    pub request_exited_message: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
}

/// Sent from the core to the test process when a non-test node exits
#[derive(Payload, Serialize, Deserialize)]
pub struct Exited {
    pub name: String,
    pub exit_code: i32,
}

/// Reply to a [`Launch`] message from the core, after the server successfully launched.
#[derive(Payload, Serialize, Deserialize)]
pub struct LaunchFinished {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
}

/// Sent from a test node to the core to start a monitoring session. This can be used by test
/// nodes to monitor message exchange between nodes whose names match the given regexes.
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

/// Sent from the test to the core in order to send a message with advanced options
///
/// the only option for now is to have the core notify the sender upon delivery of the message
/// with a [`DeliveryNotice`]
#[derive(Payload, Serialize, Deserialize)]
pub struct SendEx {
    pub delivery_notice: bool,
    pub message: Message
}

/// Sent from the core to the sender of a message when that message is delivered, if requested
/// using [`SendEx`].  distinguish multiple delivery notices, the sender can set the
/// [`id`](crate::Body) in the message body, in which case the delivery notice will have the
/// [`in_reply_to`](crate::Body) set.
#[derive(Payload, Serialize, Deserialize)]
pub struct DeliveryNotice {}

/// Sent from any node to the core in interactive mode to stop delivery of messages (to be resumed
/// by the user in the webapp)
#[derive(Payload, Serialize, Deserialize)]
pub struct Break {}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.program, self.args.join(" "))
    }
}

fn true_if_missing() -> bool {
    true
}
