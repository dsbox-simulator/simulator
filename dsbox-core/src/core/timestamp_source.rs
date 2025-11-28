use chrono::Local;
use libproto::system::event::Timestamp;

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