//! A protocol of all [`Event`]s to (potientially) reconstruct execution after the fact.
//!
//! In the future, the [`Protocol`] might also be used as the broadcast queue for [`Event`]s, since
//! the current implementation (using [`tokio`]'s [`broadcast`] implementation) permits lagging,
//! which we do not want (i.e. when reloading the webapp during execution),
//! and [`Protocol`] keeps a list of all [`Event`]s anyways...

use std::path::Path;

use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

use crate::core::event::Event;

/// A protocol of all [`Event`]s that happened during execution so far.
pub struct Protocol {
    events: Vec<Event>,
}

impl Protocol {
    /// Creates a new empty [`Protocol`]
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Starts a new [`tokio::task`] that pushes all [`Event`]s broadcast to the `receiver` onto the protocol.
    /// The returned [`JoinHandle`] can be `await`ed to get the finished [`Protocol`] back.
    pub fn collect(mut self, mut receiver: broadcast::Receiver<Event>) -> JoinHandle<Self> {
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(ev) => self.events.push(ev),
                    Err(RecvError::Lagged(_)) => {
                        log::warn!("protocol lags behind core events");
                        continue;
                    }
                    Err(RecvError::Closed) => break,
                }
            }
            self
        })
    }


    /// Writes the [`Protocol`]s [`Event`]s to the given file path (creating or truncating the file)
    /// with one line for each [`Event`].
    pub async fn write_to_file(&self, file: impl AsRef<Path>) -> std::io::Result<()> {
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file).await?;

        for event in &self.events {
            let mut line = serde_json::to_string(event).unwrap();
            line.push('\n');
            file.write_all(line.as_bytes()).await?;
        }
        Ok(())
    }
}
