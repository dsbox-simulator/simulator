//! System messages, that the core uses to communicate with its clients.
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::Message;
use crate::Payload;

/// Sent from the client process to the core when it wants to initialize a specific network
/// of multiple client- and server nodes. The `proxy` field is a command (possibly including arguments)
/// to be launched as a "proxy" for each server process. Every message from and to that server
/// will first be delivered to the "proxy", which can then decide what to do with it, and weather
/// or not it wants to forward the message or consume it.
#[derive(Payload, Serialize, Deserialize)]
pub struct Setup {
    pub clients: Vec<String>,
    pub servers: Vec<String>,
    pub proxy: Option<String>,
}

/// Reply to a [`Setup`] message from the core, after initialization completes successfully.
#[derive(Payload, Serialize, Deserialize)]
pub struct SetupOk {}

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

/// when sent from a proxy process to the core, forwards a given message
/// - if the message was a message sent **to** the server, it will then be delivered to the actual server process
/// - if the message was a message sent **from** the actual server process, it will be delivered to its destination
/// when sent from the core to the proxy, it informs the proxy that a new message was either sent
/// from the server, or should be delivered to the server
#[derive(Payload, Serialize, Deserialize)]
pub struct Proxy {
    pub message: Message,
}