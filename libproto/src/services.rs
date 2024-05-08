use serde::{Deserialize, Serialize};

use crate::Payload;

/// can be sent from a node to the core, in order to receive a [`TimerExpired`] message after the specified time has elapsed
/// Nodes can use the `msg_id` field to differentiate between different timers expiring
#[derive(Payload, Serialize, Deserialize)]
pub struct Timer {
    /// a name for the timer that should be displayed to the user in the webapp
    pub name: String,
    /// the timeout in seconds, after which the [`TimerExpired`] message should be sent
    pub seconds: f64,
}

/// sent from the core to a node, when one of its set timers expires
#[derive(Payload, Serialize, Deserialize)]
pub struct TimerExpired {}