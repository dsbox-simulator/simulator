use crate::Payload;
use serde::{Deserialize, Serialize};

/// Sent from a node to the core in interactive mode to stop delivery of messages (to be resumed
/// by the user in the webapp). This is separate from a [`Control`] Message, because it has
/// a separate capability, so that a node can be given the capability to break execution
/// without giving it the ability to control the execution outright.
#[derive(Payload, Serialize, Deserialize)]
pub struct Break {}

/// Sent from a node to the core to control the execution
#[derive(Payload, Serialize, Deserialize, Debug)]
#[serde(tag = "command")]
pub enum Control {
    /// Pauses the delivery of [`Message`](crate::Message)s in the [`Core`].
    Break,
    /// Executes a single step. the [`Core`] will deliver a single [`Message`](crate::Message) and then pause again.
    Step,
    /// Resumes execution normally.
    Resume,
    /// delivers a message form the network with the given sent timestamp
    Deliver { sent_timestamp: usize },
    /// drops a message form the network with the given sent timestamp
    Drop { sent_timestamp: usize },
    /// instructs the core to shut down
    Shutdown,
}

