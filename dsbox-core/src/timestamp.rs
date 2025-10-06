//! Unique physical and logical timestamps.
//!
//! [`Timestamp`]s are used to uniquely stamp events (or messages, etc.) with a logical timestamp
//! (implemented as a [`usize`] counter) and a physical timestamp (implemented using [`chrono`])

use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// A logical and physical timestamp.
///
/// Logical timestamps is always strictly increasing for each [`Timestamp`] created (with [`Timestamp::now`]),
/// whereas physical timestamps adhere to whatever [`chrono`] specifies.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Timestamp {
    /// the logical timestamp, strictly increasing for each newly created [`Timestamp`].
    pub logical: usize,
    /// the physical timestamp, time-zone aware and created with [`Local::now`].
    pub physical: DateTime<Local>,
}

/// the counter used to generate logical timestamps
static LOGICAL_CLOCK: AtomicUsize = AtomicUsize::new(0);

impl Timestamp {
    /// Creates a new [`Timestamp`] with a new and unique logical timestamp with the physical local system time.
    pub fn now() -> Self {
        Self {
            logical: LOGICAL_CLOCK.fetch_add(1, Ordering::SeqCst),
            physical: Local::now(),
        }
    }
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