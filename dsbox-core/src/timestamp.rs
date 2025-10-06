//! Unique physical and logical timestamps.
//!
//! [`Timestamp`]s are used to uniquely stamp events (or messages, etc.) with a logical timestamp
//! (implemented as a [`usize`] counter) and a physical timestamp (implemented using [`chrono`])

use std::fmt::{Display, Formatter};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// A source of timestamps, with a strictly increasing logical clock
pub struct TimestampSource {
    logical_now: usize,
}

impl TimestampSource {
    pub fn new() -> Self {
        Self { logical_now: 0 }
    }

    /// Creates a new [`Timestamp`] with a new and unique logical timestamp for this [`TimestampSource`] with the physical local system time.
    pub fn now(&mut self) -> Timestamp {
        let ts = Timestamp {
            logical: self.logical_now,
            physical: Local::now(),
        };
        self.logical_now += 1;
        ts
    }
}

/// A logical and physical timestamp.
///
/// Logical timestamps is always strictly increasing for each [`Timestamp`] created (with [`TimestampSource::now`]),
/// whereas physical timestamps adhere to whatever [`chrono`] specifies.
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

impl Display for Timestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.physical)
    }
}
