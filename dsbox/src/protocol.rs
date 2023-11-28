use std::path::Path;

use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

use crate::core::event::Event;

pub struct Protocol {
    events: Vec<Event>,
}

impl Protocol {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

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
