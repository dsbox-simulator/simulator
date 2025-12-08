//! Events that can be used to track the execution of the simulation.
//!
//! An [`Event`] in this case is anything that happens during the execution of the simulation, including (for now):
//!
//! - the state of the nodes (launching and terminating)
//! - the sending and delivering of [`Message`]s
//! - log lines that are written by nodes
//!
//! Other events may be added in the future.
//! These events are published by a running core.

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use crate::Payload;
use crate::services::LogMessage;
use crate::Message;

/// Sent from a node to the core in order to receive a complete protocol of every event
/// happening in the core
#[derive(Payload, Serialize, Deserialize)]
pub struct SubscribeEvents {}

/// Sent from the core to a node that has subscribed to events via a [`SubscribeEvents`] message
#[derive(Payload, Serialize, Deserialize)]
pub struct PublishEvent {
    #[serde(flatten)]
    pub event: Event,
}

/// Describes a single event (in [`Event::data`]) with a timestamp (in [`Event::timestamp`])
#[derive(Clone, Serialize, Deserialize)]
pub struct Event {
    /// the timestamp at which this event occurred. The [`Timestamp::logical`] is sometimes used
    /// as an identifier of this [`Event`], as it is always unique.
    pub timestamp: Timestamp,
    /// the specifics of the [`Event`] (what has happened)
    pub data: EventData,
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
        msg: Message,
    },
    /// Emitted when a [`Message`] is delivered
    DeliverMessage {
        /// the logical timestamp when the [`Message`] was sent. Since these are unique,
        /// this sufficient to identify the specific [`Message`] that was delivered
        sent_timestamp: usize,
    },
    /// Emitted when a [`Message`] is dropped (i.e. by the webapp)
    DropMessage {
        /// the logical timestamp when the [`Message`] was sent. Since these are unique,
        /// this sufficient to identify the specific [`Message`] that was dropped
        sent_timestamp: usize,
    },
    /// Emitted after a process exited
    NodeDisconnected {
        /// the id of the process that exited. See [`NodeId`]
        name: String,
        /// the exit code of that node.
        exit_code: Option<i32>,
    },
    /// Emitted after a process is started
    NodeLaunched {
        /// the name of the node
        name: String,
        /// the commandline (executable + arguments) that was used to launch the process
        commandline: String,
    },
    /// Emitted when a node logs a line
    Log {
        /// the name of the node that logged a line
        node: String,
        /// the log message and possible marker
        message: LogMessage,
    },
}

impl Event {
    /// creates a new [`Event`] with the given timestamp and data
    fn new(timestamp: Timestamp, data: EventData) -> Self {
        Self { timestamp, data }
    }

    /// creates a new [`Event`] with the given timestamp and [`EventData::Reset`]
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

    /// creates a new [`Event`] with the given timestamp and [`EventData::DropMessage`]
    pub fn drop_message(timestamp: Timestamp, sent_timestamp: usize) -> Self {
        Self::new(timestamp, EventData::DropMessage { sent_timestamp })
    }

    /// creates a new [`Event`] with the given timestamp and [`EventData::NodeDisconnected`]
    pub fn node_launched(
        timestamp: Timestamp,
        name: String,
        commandline: String,
    ) -> Self {
        Self::new(
            timestamp,
            EventData::NodeLaunched {
                name,
                commandline,
            },
        )
    }

    /// creates a new [`Event`] with the given timestamp and [`EventData::NodeDisconnected`]
    pub fn node_disconnected(timestamp: Timestamp, name: String, exit_code: Option<i32>) -> Self {
        Self::new(timestamp, EventData::NodeDisconnected { name, exit_code })
    }

    /// creates a new [`Event`] with the given timestamp and [`EventData::Log`]
    pub fn log(timestamp: Timestamp, name: String, message: LogMessage) -> Self {
        Self::new(timestamp, EventData::Log { node: name, message })
    }
}

/// A logical and physical timestamp.
///
/// Logical timestamps must always be strictly increasing for each [`Timestamp`] created,
/// whereas physical timestamps may adhere to whatever [`chrono`] specifies.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Timestamp {
    /// the logical timestamp, strictly increasing for each newly created [`Timestamp`].
    pub logical: usize,
    /// the physical timestamp, time-zone aware and created with [`Local::now`].
    pub physical: DateTime<Local>,
}

impl PartialEq<Self> for Timestamp {
    fn eq(&self, other: &Self) -> bool {
        self.logical == other.logical
    }
}

impl Eq for Timestamp {}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.logical.partial_cmp(&other.logical)
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.logical.cmp(&other.logical)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.physical)
    }
}