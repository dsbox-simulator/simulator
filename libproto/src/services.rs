use serde::{Deserialize, Serialize};

use crate::Payload;

/// can be sent from a node to the core, in order to receive a [`TimerExpired`] message after the specified time has elapsed
/// Nodes can use the `id` field to differentiate between different timers expiring
#[derive(Payload, Serialize, Deserialize)]
pub struct Timer {
    /// a name for the timer that should be displayed to the user in the webapp
    pub name: String,
    /// the timeout in seconds, after which the [`TimerExpired`] message should be sent
    pub seconds: f64,
}

/// sent from the core to a node, when one of its set timers expires
#[derive(Payload, Serialize, Deserialize)]
pub struct TimerExpired {
    pub name: String,
}

/// can be sent from a node to the core as an alternative means (alternative to stderr) to send log
/// messages. This way, additional information can be attached to a log message (e.g. if the message
/// should have a marker attached on the timeline).
/// Multiline log messages are also possible this way.
#[derive(Payload, Serialize, Deserialize, Clone)]
pub struct LogMessage {
    pub text: String,
    pub marker: Option<LogMarker>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LogMarker {
    pub label: String,
    pub color: Option<LogMarkerColor>,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum LogMarkerColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}