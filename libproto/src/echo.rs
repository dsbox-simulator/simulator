//! Message bodies for a simple echo/reply test.
use serde::{Deserialize, Serialize};
use crate::Payload;

/// Sent to a node, which is expected to reply with an [`EchoOk`] message.
#[derive(Payload, Serialize, Deserialize)]
pub struct Echo {
    /// An arbitrary [`String`] expected to be returned back from the destination node
    pub echo: String,
}

/// Reply to an [`Echo`] message.
#[derive(Payload, Serialize, Deserialize)]
pub struct EchoOk {
    /// The same [`String`] that was in the corresponding [`Echo`] message.
    pub echo: String,
}