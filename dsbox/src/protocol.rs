//! A protocol of all [`Event`]s as a broadcast channel.

use std::sync::Arc;

use tokio::sync::{Notify, RwLock};

use crate::core::event::Event;

/// A protocol of all [`Event`]s that happened during execution so far.
/// Is used to as a publish/subscribe broadcast channel, that holds a list of all previous
/// events, so that new subscribers can receive those after the fact.
pub struct Protocol {
    /// Reference to shared state between the [`Protocol`] and its [`ProtocolSubscriber`]s.
    inner: Arc<SharedProtocolHolder>,
}

/// Shared state between the [`Protocol`] and [`ProtocolSubscriber`]s.
struct SharedProtocolHolder {
    /// List of all events that occurred.
    events: RwLock<Vec<Event>>,
    /// Used to notify subscribers, that new events are available.
    new_events: Notify,
}

/// Receives published events from the protocol.
pub struct ProtocolSubscriber {
    /// Reference to shared state between the [`ProtocolSubscriber`] and the [`Protocol`].
    inner: Arc<SharedProtocolHolder>,
    /// the index of the event in the protocol that should be received next by this subscriber.
    next_event_index: usize,
}

impl Protocol {
    /// Creates a new empty [`Protocol`]
    pub fn new() -> Self {
        Self { inner: Arc::new(SharedProtocolHolder { events: RwLock::new(Vec::new()), new_events: Notify::new() }) }
    }

    /// Publishes an [`Event`] for all [`ProtocolSubscriber`]s to receive. [`Event`]s are buffered
    /// indefinitely, so that new [`ProtocolSubscribers`]s can receive all old events.
    pub fn publish_event(&mut self, event: Event) {
        self.inner.events.blocking_write().push(event);
        self.inner.new_events.notify_waiters();
    }

    /// Creates a new [`ProtocolSubscriber`] that will receive [`Event`]s published through this [`Protocol`].
    pub fn subscribe(&self) -> ProtocolSubscriber {
        ProtocolSubscriber {
            inner: self.inner.clone(),
            next_event_index: 0,
        }
    }
}

impl ProtocolSubscriber {
    /// Waits for a single (new) [`Event`] and returns it.
    pub async fn recv(&mut self) -> Event {
        loop {
            if let Some(event) = self.inner.events.read().await.get(self.next_event_index) {
                self.next_event_index += 1;
                return event.clone();
            }
            self.inner.new_events.notified().await;
        }
    }

    /// Creates a new [`ProtocolSubscriber`] that will receive [`Event`]s published through the same [`Protocol`],
    /// but will also (re-)receive all past [`Event`]s.
    pub fn resubscribe(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            next_event_index: 0,
        }
    }
}