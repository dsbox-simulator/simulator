use std::collections::VecDeque;

use libproto::Message;

use crate::timestamp::Timestamp;

pub struct Network {
    messages_in_transit: VecDeque<(Timestamp, Message)>,
}

impl Network {
    pub fn new() -> Self {
        Self { messages_in_transit: VecDeque::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.messages_in_transit.is_empty()
    }

    pub fn insert(&mut self, timestamp: Timestamp, message: Message) {
        match self.message_by_timestamp(timestamp.logical) {
            Ok(_) => panic!("tried to insert message into network with same timestamp twice"),
            Err(idx) => self.messages_in_transit.insert(idx, (timestamp, message))
        }
    }

    pub fn remove_oldest(&mut self) -> Option<(Timestamp, Message)> {
        self.messages_in_transit.pop_front()
    }

    pub fn remove_one(&mut self, logical_timestamp: usize) -> Option<(Timestamp, Message)> {
        if let Ok(idx) = self.message_by_timestamp(logical_timestamp) {
            self.messages_in_transit.remove(idx)
        } else {
            None
        }
    }

    fn message_by_timestamp(&self, logical_timestamp: usize) -> Result<usize, usize> {
        self.messages_in_transit.binary_search_by_key(&logical_timestamp, |(t, _)| t.logical)
    }
}