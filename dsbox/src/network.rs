//! Manages [`Message`]s in transit.
//!
//! All [`Message`]s in the network simulation first enter the network (when the are sent),
//! and at a later point are removed from the network (and then delivered).
//! The [`Network`] struct is used to manage all [`Message`]s that are currently in the network (i.e in transit)

use std::collections::VecDeque;

use libproto::Message;

use crate::timestamp::Timestamp;


/// holds all [`Message`]s that are in transit, ordered by the timestamp they are sent.
/// [`Message`]s can be removed in FIFO order, or one-by-one using the logical timestamp as a key
pub struct Network {
    /// all [`Message`]s and their corresponding timestamps, that are currently in the network.
    ///
    /// this queue is always ordered by timestamp (from oldest to newest) as ensured by [`Network::insert`].
    messages_in_transit: VecDeque<(Timestamp, Message)>,
}

impl Network {
    /// creates a new (empty) network
    pub fn new() -> Self {
        Self { messages_in_transit: VecDeque::new() }
    }

    /// returns `true` if there are no [`Message`]s in transit
    pub fn is_empty(&self) -> bool {
        self.messages_in_transit.is_empty()
    }

    /// inserts a new [`Message`] into the network, with the given timestamp,
    /// # Panics
    /// Panics if a [`Message`] with the same timestamp is already in the network.
    /// If timestamps are created using [`Timestamp::now`] for each [`Message`], then this should never happen
    pub fn insert(&mut self, timestamp: Timestamp, message: Message) {
        match self.message_by_timestamp(timestamp.logical) {
            Ok(_) => panic!("tried to insert message into network with same timestamp twice"),
            Err(idx) => self.messages_in_transit.insert(idx, (timestamp, message))
        }
    }

    /// Removes and returns the [`Message`] with the oldest timestamp (i.e. in FIFO order regarding the timestamps).
    /// Returns `None` if there are no [`Message`]s in the network.
    pub fn remove_oldest(&mut self) -> Option<(Timestamp, Message)> {
        self.messages_in_transit.pop_front()
    }

    /// Removes and returns the [`Message`] with the given logical timestamp, or `None` if no message with that timestamp is in the network.
    pub fn remove_one(&mut self, logical_timestamp: usize) -> Option<(Timestamp, Message)> {
        if let Ok(idx) = self.message_by_timestamp(logical_timestamp) {
            self.messages_in_transit.remove(idx)
        } else {
            None
        }
    }

    /// Returns [`Result::Ok`] with the index of the [`Message`] in the queue that has the given timestamp, if it exists.
    /// Otherwise Returns [`Result::Err`] with the index in the queue where a [`Message`] with the given timestamp should be inserted.
    fn message_by_timestamp(&self, logical_timestamp: usize) -> Result<usize, usize> {
        self.messages_in_transit.binary_search_by_key(&logical_timestamp, |(t, _)| t.logical)
    }
}