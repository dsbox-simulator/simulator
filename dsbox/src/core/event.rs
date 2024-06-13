//! Events that can be used to track the execution of the simulation.
//!
//! An [`Event`] in this case is anything that happens during the execution of the simulation, including (for now):
//!
//! - the setup of the system (a list of nodes by their names)
//! - the sending and delivering of [`Message`]s
//! - the disconnection of a node
//! - log lines that are written by nodes
//!
//! Other events may be added in the future.
//! These events are published by the running [`Core`](super::Core).

use serde::{Deserialize, Serialize};

use libproto::Message;
use libproto::services::LogMessage;
use crate::core::node::NodeId;

use crate::timestamp::Timestamp;

/// Describes a single event (in [`Event::data`]) with a timestamp (in [`Event::timestamp`])
#[derive(Clone, Serialize, Deserialize)]
pub struct Event {
    /// the timestamp at which this event occurred. The [`Timestamp::logical`] is sometimes used
    /// as an identifier of this [`Event`], as it is always unique.
    timestamp: Timestamp,
    /// the specifics of the [`Event`] (what has happened)
    data: EventData,
}

/// Contains information about a single event (what has happened)
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum EventData {
    /// Emitted when a new test run is started.
    Reset,

    /// Emitted when a [`Message`] is sent
    SendMessage {
        /// the [`Message`] that was sent. The sender of this message was validated at this point
        /// but the receiver is only validated when the [`Message`] is delivered
        msg: Message
    },
    /// Emitted when a [`Message`] is delivered
    DeliverMessage {
        /// the logical timestamp when the [`Message`] was sent. Since these are unique,
        /// this sufficient to identify the specific [`Message`] that was delivered
        sent_timestamp: usize
    },
    /// Emitted after a process exited
    NodeDisconnected {
        /// the id of the process that exited. See [`crate::core::ProcessManager`]
        id: NodeId
    },
    /// Emitted after a process is started
    NodeLaunched {
        /// the id of the process that started. See [`crate::core::ProcessManager`]
        id: NodeId,
        /// the name of the node
        name: String,
        /// the commandline (executable + arguments) that was used to launch the process
        commandline: String,
    },
    /// Emitted when a node logs a line
    Log {
        /// the id of the process that logged a line. See [`crate::core::ProcessManager`]
        id: NodeId,
        /// the log message and possible marker
        message: LogMessage,
    },
}

impl Event {
    /// creates a new [`Event`] with the given timestamp and data
    fn new(timestamp: Timestamp, data: EventData) -> Self {
        Self {
            timestamp,
            data,
        }
    }

    /// creates a new [`Event`] with the given timestamp and [`EventData::Setup`]
    pub fn reset(timestamp: Timestamp) -> Self {
        Self::new(timestamp, EventData::Reset)
    }


    /// creates a new [`Event`] with the given timestamp and [`EventData::SendMessage`]
    pub fn send_message(timestamp: Timestamp, msg: Message) -> Self {
        Self::new(timestamp, EventData::SendMessage { msg })
    }

    /// creates a new [`Event`] with the given timestamp and [`EventData::DeliverMessage`]
    pub fn deliver_message(timestamp: Timestamp, sent_timestamp: usize) -> Self {
        Self::new(timestamp, EventData::DeliverMessage { sent_timestamp })
    }

    /// creates a new [`Event`] with the given timestamp and [`EventData::NodeDisconnected`]
    pub fn node_launched(timestamp: Timestamp, id: NodeId, name: String, commandline: String) -> Self {
        Self::new(timestamp, EventData::NodeLaunched { id, name, commandline })
    }

    /// creates a new [`Event`] with the given timestamp and [`EventData::NodeDisconnected`]
    pub fn node_disconnected(timestamp: Timestamp, id: NodeId) -> Self {
        Self::new(timestamp, EventData::NodeDisconnected { id })
    }

    /// creates a new [`Event`] with the given timestamp and [`EventData::Log`]
    pub fn log(timestamp: Timestamp, id: NodeId, message: LogMessage) -> Self {
        Self::new(timestamp, EventData::Log { id, message })
    }
}