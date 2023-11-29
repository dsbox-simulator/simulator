//! A protocol of all [`Event`]s to (potientially) reconstruct execution after the fact.
//!
//! In the future, the [`Protocol`] might also be used as the broadcast queue for [`Event`]s, since
//! the current implementation (using [`tokio`]'s [`broadcast`] implementation) permits lagging,
//! which we do not want (i.e. when reloading the webapp during execution),
//! and [`Protocol`] keeps a list of all [`Event`]s anyways...

use std::sync::Arc;

use tokio::sync::{Notify, RwLock};

use crate::core::event::Event;

/// A protocol of all [`Event`]s that happened during execution so far.
pub struct Protocol {
    inner: Arc<SharedProtocolHolder>,
}

struct SharedProtocolHolder {
    events: RwLock<Vec<Event>>,
    new_events: Notify,
}

pub struct ProtocolSubscriber {
    inner: Arc<SharedProtocolHolder>,
    last_event_read: usize,
}

impl Protocol {
    /// Creates a new empty [`Protocol`]
    pub fn new() -> Self {
        Self { inner: Arc::new(SharedProtocolHolder { events: RwLock::new(Vec::new()), new_events: Notify::new() }) }
    }

    pub fn publish_event(&mut self, event: Event) {
        self.inner.events.blocking_write().push(event);
        self.inner.new_events.notify_waiters();
    }

    pub fn subscribe(&self) -> ProtocolSubscriber {
        ProtocolSubscriber {
            inner: self.inner.clone(),
            last_event_read: 0,
        }
    }
}

impl ProtocolSubscriber {
    pub async fn recv(&mut self) -> Event {
        loop {
            if let Some(event) = self.inner.events.read().await.get(self.last_event_read) {
                self.last_event_read += 1;
                return event.clone();
            }
            self.inner.new_events.notified().await;
        }
    }

    pub fn resubscribe(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            last_event_read: 0,
        }
    }
}