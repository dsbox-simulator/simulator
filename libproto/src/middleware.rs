use serde::{Deserialize, Serialize};

use crate::Payload;

use crate::Message;

/// Sent from the core to a middleware process when a message gets sent from a process
/// that sits below in the middleware stack
#[derive(Payload, Serialize, Deserialize)]
pub struct Forward {
    pub message: Message,
}

/// Sent from a middleware process to the core to deliver the message to the next
/// process below in the middleware stack
#[derive(Payload, Serialize, Deserialize)]
pub struct Next {
    pub message: Message,
}