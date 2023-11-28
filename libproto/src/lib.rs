use std::fmt::{Debug, Display, Formatter};
use std::io::stdin;

use serde::{Deserialize, Serialize};

pub use crate::payload::Payload;
pub use payload_derive::Payload;

mod payload;
pub mod init;
pub mod echo;
#[cfg(feature = "system_messages")]
pub mod system;

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub src: String,
    #[serde(rename = "dest")]
    pub dst: String,
    pub body: Body,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Body {
    #[serde(rename = "type")]
    pub ty: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<usize>,

    #[serde(flatten)]
    pub data: serde_json::Value,
}

impl Message {
    pub fn recv() -> Option<Result<Self, std::io::Error>> {
        Message::recv_iter().next()
    }

    pub fn recv_iter() -> impl Iterator<Item=Result<Self, std::io::Error>> {
        stdin().lines().map(|line| {
            Ok(Self::from_json(&line?)?)
        })
    }

    pub fn from_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }

    pub fn new<P: Payload>(src: &str, dst: &str, msg_id: Option<usize>, payload: P) -> Self {
        let data = serde_json::to_value(payload)
            .expect("failed to convert payload to json value");

        Self {
            src: src.to_owned(),
            dst: dst.to_owned(),
            body: Body {
                ty: P::TYPE.to_owned(),
                msg_id,
                in_reply_to: None,
                data,
            },
        }
    }

    pub fn reply<P: Payload>(&self, msg_id: Option<usize>, ty: P) -> Self {
        let mut message = Self::new(&self.dst, &self.src, msg_id, ty);
        message.body.in_reply_to = self.body.msg_id;
        message
    }

    pub fn payload<P: Payload>(&self) -> Result<P, PayloadError> {
        if self.body.ty != P::TYPE { return Err(PayloadError::MismatchedType(P::TYPE, self.body.ty.clone())); }
        serde_json::from_value(self.body.data.clone())
            .map_err(|e| PayloadError::DeserializeError(P::TYPE, e))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self)
            .expect("failed to serialize message")
    }

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

#[derive(Debug)]
pub enum PayloadError {
    MismatchedType(&'static str, String),
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