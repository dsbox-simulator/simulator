use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use libproto::Message;

use crate::timestamp::Timestamp;

#[derive(Clone, Serialize, Deserialize)]
pub struct Event {
    timestamp: Timestamp,
    data: EventData,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum EventData {
    Setup { nodes: HashMap<String, usize> },
    SendMessage { msg: Message },
    DeliverMessage { sent_timestamp: usize },
    NodeDisconnected { node_id: usize },
    Log { node_id: usize, source_file: PathBuf, line: String },
}


impl Event {
    fn new(timestamp: Timestamp, data: EventData) -> Self {
        Self {
            timestamp,
            data,
        }
    }

    pub fn setup(timestamp: Timestamp, nodes: HashMap<String, usize>) -> Self {
        Self::new(timestamp, EventData::Setup { nodes })
    }

    pub fn send_message(timestamp: Timestamp, msg: Message) -> Self {
        Self::new(timestamp, EventData::SendMessage { msg })
    }

    pub fn deliver_message(timestamp: Timestamp, sent_timestamp: usize) -> Self {
        Self::new(timestamp, EventData::DeliverMessage { sent_timestamp })
    }

    pub fn node_disconnected(timestamp: Timestamp, node_id: usize) -> Self {
        Self::new(timestamp, EventData::NodeDisconnected { node_id })
    }

    pub fn log(timestamp: Timestamp, node_id: usize, source_file: PathBuf, line: String) -> Self {
        Self::new(timestamp, EventData::Log { node_id, source_file, line })
    }
}