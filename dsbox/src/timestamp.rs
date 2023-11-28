use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Timestamp {
    pub logical: usize,
    pub physical: DateTime<Local>,
}

static LOGICAL_CLOCK: AtomicUsize = AtomicUsize::new(0);

impl Timestamp {
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