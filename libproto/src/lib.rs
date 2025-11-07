//! this crate is used to share the [`Message`] struct between the dsbox binary code and
//! some node implementations that are written in rust. It is a convenience to avoid
//! duplicating code, but it is also used to specify the (JSON) message format that
//! drives communication in the simulated system.

use std::fmt::{Debug, Display, Formatter};
use std::io::stdin;

use serde::{Deserialize, Serialize};

pub use payload_derive::Payload;

pub use crate::payload::Payload;

mod payload;
pub mod init;
#[cfg(feature = "system_messages")]
pub mod system;
#[cfg(feature = "middleware")]
pub mod middleware;
pub mod services;


/// A single message that can be sent between the nodes
#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    /// The name of the node that sent the message. This can be used to "reply" to a message,
    /// and it is also validated in the core (no node can send messages with a `src` other than itself).
    pub src: String,
    /// The name of the node that the message is meant for.
    #[serde(rename = "dest")]
    pub dst: String,
    /// The contents of the message
    pub body: Body,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Body {
    /// The type of the message. This can be used to identify and deserialize the structure of the messages `data`.
    /// Some types are defined in this crate, but nodes may also define their own message types to communicate between one another.
    #[serde(rename = "type")]
    pub ty: String,

    /// An optional id for the message. This id can be determined by the sender. A receiver can reply to a message and use this field
    /// as an identifier, so that the original sender can figure out what message the reply is referring to. See [`Body::in_reply_to`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<usize>,

    /// An optional id that identifies a message that this message is a reply to. See [`Body::msg_id`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<usize>,

    /// Data associated with this message. This can vary from message to message, specified by its type. See [`Body::ty`]
    #[serde(flatten)]
    pub data: serde_json::Value,
}

impl Message {
    /// Helper function for nodes to "receive" a message (reads from `stdin`)
    pub fn recv() -> Option<Result<Self, std::io::Error>> {
        Message::recv_iter().next()
    }

    /// Helper function for nodes iterate over all messages "received" via `stdin`
    pub fn recv_iter() -> impl Iterator<Item=Result<Self, std::io::Error>> {
        stdin().lines().map(|line| {
            Ok(Self::from_json(&line?)?)
        })
    }

    /// Deserializes a (JSON) string into a [`Message`]
    pub fn from_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }

    /// Create a new message with a given source, destination, optional id and a [`Payload`].
    pub fn new<P: Payload>(src: &str, dst: &str, msg_id: Option<usize>, payload: P) -> Self {
        let data = serde_json::to_value(payload)
            .expect("failed to convert payload to json value");

        Self {
            src: src.to_owned(),
            dst: dst.to_owned(),
            body: Body {
                ty: P::TYPE.to_owned(),
                id: msg_id,
                in_reply_to: None,
                data,
            },
        }
    }

    /// Creates a message with a [`Payload`] that is a reply to `self`
    /// (swaps source and destination, and sets the `in_reply_to` field if `self` has a `msg_id`).
    pub fn reply<P: Payload>(&self, msg_id: Option<usize>, ty: P) -> Self {
        let mut message = Self::new(&self.dst, &self.src, msg_id, ty);
        message.body.in_reply_to = self.body.id;
        message
    }

    /// Attempts to deserialize the message body into the given [`Payload`] type.
    /// Checks if the message type matches the [`Payload`] type and the deserializes the body into that type.
    /// Returns [`Result::Ok`] if the type matches and the body could be deserialized, and returns
    /// [`Result::Err`] if the type does not match or there was an error deserializing the body.
    pub fn payload<P: Payload>(&self) -> Result<P, PayloadError> {
        if self.body.ty != P::TYPE { return Err(PayloadError::MismatchedType(P::TYPE, self.body.ty.clone())); }
        serde_json::from_value(self.body.data.clone())
            .map_err(|e| PayloadError::DeserializeError(P::TYPE, e))
    }

    /// Serializes `self` into a (JSON) [`String`].
    pub fn to_json(&self) -> String {
        serde_json::to_string(self)
            .expect("failed to serialize message")
    }

    /// Helper function for nodes to "send" [`Message`]s. (writes them to `stdout`).
    pub fn send(&self) {
        println!("{}", self.to_json())
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_json())
    }
}

impl Debug for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.to_json())
    }
}

/// Error that may occur when trying to deserialize a message body
#[derive(Debug)]
pub enum PayloadError {
    /// The type of the message does no match the type it should have been deserialized into.
    /// Contains the type was expected (first parameter) and the type the message actually had (second parameter).
    MismatchedType(&'static str, String),
    /// The type matches, but an error occurred during deserialization.
    /// Contains the type of the message, and the error that occurred.
    DeserializeError(&'static str, serde_json::Error),
}

impl Display for PayloadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PayloadError::MismatchedType(expected, got) => write!(f, "failed to deserialize payload: mismatched type: expected `{expected}`, got `{got}`"),
            PayloadError::DeserializeError(expected, err) => write!(f, "failed to deserialize payload of type `{expected}`: {err}"),
        }
    }
}

impl std::error::Error for PayloadError {}