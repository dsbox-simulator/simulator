//! Manages [`Message`]s in transit.
//!
//! All [`Message`]s in the network simulation first enter the network (when they are sent),
//! and at a later point are removed from the network (and then delivered).
//! The [`Network`] struct is used to manage all [`Message`]s that are currently in the network (i.e. in transit)

use std::collections::{HashSet, VecDeque};

use libproto::Message;

use crate::core::node::NodeId;
use crate::timestamp::Timestamp;

/// holds all [`Message`]s that are in transit, ordered by the timestamp they are sent.
/// [`Message`]s can be removed in FIFO order, or one-by-one using the logical timestamp as a key
pub struct Network {
    /// all [`Message`]s and their corresponding timestamps, that are currently in the network.
    ///
    /// this queue is always ordered by timestamp (from oldest to newest) as ensured by [`Network::insert`].
    messages_in_transit: VecDeque<MessageInTransit>,
    /// describes the order in which messages are removed from the network
    pub network_order: NetworkOrder
}

/// describes the order in which messages are removed from the network to be delivered by the core
#[derive(Clone, Copy)]
pub enum NetworkOrder {
    /// always remove messages from the network in the same order they were put in (i.e. FIFO)
    Fifo,
    /// remove messages in random order from the network, but respecting FIFO order
    /// per "channel". A "channel" is a pair of source and destination. This means that if a node
    /// sends two messages to a single other node, these messages are kept in the order they were sent,
    /// all other messages may be reordered.
    RandomFifoChannels
}

/// A single message that is currently "in transit" in the network
/// including additional information
pub struct MessageInTransit {
    /// timestamp when this message was sent
    pub sent_timestamp: Timestamp,
    /// the message itself
    pub message: Message,
    /// the id of the node that sent the message, unless it was sent by the core itself
    pub source: Option<NodeId>,
    /// weather a delivery notice was requested
    pub delivery_notice: DeliveryNotice,
}

/// Request of a delivery notice for a message
pub enum DeliveryNotice {
    /// no delivery notice was requested
    None,
    /// a delivery notice was requested, and the reply should contain the given id
    WithReplyId(Option<usize>),
}

impl Network {
    /// creates a new (empty) network
    pub fn new(network_order: NetworkOrder) -> Self {
        Self {
            messages_in_transit: VecDeque::new(),
            network_order
        }
    }

    /// returns `true` if there are no [`Message`]s in transit
    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.messages_in_transit.is_empty()
    }

    /// inserts a new [`Message`] into the network, with the given timestamp,
    ///
    /// # Panics
    ///
    /// Panics if a [`Message`] with the same timestamp is already in the network.
    /// If timestamps are created using [`TimestampSource::now`] for each [`Message`], then this should never happen
    pub fn insert(
        &mut self,
        timestamp: Timestamp,
        source_id: Option<NodeId>,
        message: Message,
        delivery_notice: DeliveryNotice,
    ) {
        match self.message_by_timestamp(timestamp.logical) {
            Ok(_) => panic!("tried to insert message into network with same timestamp twice"),
            Err(idx) => self.messages_in_transit.insert(
                idx,
                MessageInTransit {
                    sent_timestamp: timestamp,
                    source: source_id,
                    message,
                    delivery_notice,
                },
            ),
        }
    }

    /// returns `true` if there are remaining messages in the network from the given [`NodeId`]
    pub fn has_remaining_messages(&self, source_id: NodeId) -> bool {
        self.messages_in_transit
            .iter()
            .any(|m| m.source.is_some_and(|id| id == source_id))
    }

    /// removes and return a [`Message`] from the network, according to the currently set [`NetworkOrder`]
    pub fn remove_next(&mut self) -> Option<MessageInTransit> {
        match self.network_order {
            NetworkOrder::Fifo => self.remove_next_oldest(),
            NetworkOrder::RandomFifoChannels => self.remove_next_random()
        }
    }

    /// Removes and returns the [`Message`] with the oldest timestamp (i.e. in FIFO order regarding the timestamps).
    /// Returns `None` if there are no [`Message`]s in the network.
    pub fn remove_next_oldest(&mut self) -> Option<MessageInTransit> {
        self.messages_in_transit.pop_front()
    }

    /// removes and returns a [`Message`] from the network in random order, but respecting FIFO order
    /// per "channel". A "channel" is a pair of source and destination. This means that if a node
    /// sends two messages to a single other node, these messages are kept in the order they were sent,
    /// all other messages may be reordered.
    /// Returns `None` if there are no [`Message`]s in the network.
    pub fn remove_next_random(&mut self) -> Option<MessageInTransit> {
        if self.messages_in_transit.is_empty() {
            return None;
        }
        let mut channels = HashSet::new();
        let mut candidates = Vec::with_capacity(self.messages_in_transit.len());
        for (idx, m) in self.messages_in_transit.iter().enumerate() {
            let channel = (&m.message.src, &m.message.dest);
            if !channels.insert(channel) {
                continue;
            }
            candidates.push(idx);
        }
        let idx = candidates[rand::random_range(0..candidates.len())];
        self.messages_in_transit.remove(idx)
    }

    /// Removes and returns the [`Message`] with the given logical timestamp, or `None` if no message with that timestamp is in the network.
    #[allow(unused)]
    pub fn remove_one(&mut self, logical_timestamp: usize) -> Option<MessageInTransit> {
        if let Ok(idx) = self.message_by_timestamp(logical_timestamp) {
            self.messages_in_transit.remove(idx)
        } else {
            None
        }
    }

    /// Returns [`Ok`] with the index of the [`Message`] in the queue that has the given timestamp, if it exists.
    /// Otherwise, Returns [`Err`] with the index in the queue where a [`Message`] with the given timestamp should be inserted.
    fn message_by_timestamp(&self, logical_timestamp: usize) -> Result<usize, usize> {
        self.messages_in_transit
            .binary_search_by_key(&logical_timestamp, |m| m.sent_timestamp.logical)
    }
}
