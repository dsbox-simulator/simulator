//! Commands used to control the execution of the simulation

use crate::core::error::CoreError;
use crate::core::event::Event;
use crate::core::{Core, CoreState};

/// A command for the [`Core`] to control its execution
#[derive(Debug)]
pub enum RemoteCommand {
    /// Pauses the delivery of [`Message`](libproto::Message)s in the [`Core`].
    Break,
    /// Executes a single step. the [`Core`] will deliver a single [`Message`](libproto::Message) and then pause again.
    Step,
    /// Resumes execution normally.
    Resume,
    /// delivers a message form the network with the given sent timestamp
    Deliver(usize),
    /// drops a message form the network with the given sent timestamp
    Drop(usize),
    /// instructs the core to shut down
    Shutdown,
}

impl Core {
    /// handles a single [`RemoteCommand`]
    pub async fn handle_command(&mut self, command: RemoteCommand) -> Result<(), CoreError> {
        log::trace!("handle_command: {command:?}");
        match command {
            RemoteCommand::Break => self.set_state(CoreState::Paused),
            RemoteCommand::Step => self.set_state(CoreState::Stepping),
            RemoteCommand::Resume => self.set_state(CoreState::Running),
            RemoteCommand::Deliver(sent_timestamp) => {
                self.deliver_by_timestamp(sent_timestamp).await?
            }
            RemoteCommand::Drop(sent_timestamp) => self.drop_by_timestamp(sent_timestamp).await,
            RemoteCommand::Shutdown => {
                // terminate all nodes
                self.terminate(|_| true).await;
            }
        }
        Ok(())
    }

    fn set_state(&mut self, state: CoreState) {
        self.state = state;
    }

    async fn drop_by_timestamp(&mut self, sent_timestamp: usize) {
        self.network.remove_one(sent_timestamp);
        self.event_sender
            .send(Event::drop_message(
                self.timestamp_source.now(),
                sent_timestamp,
            ))
            .await
            .ok();
    }

    async fn deliver_by_timestamp(&mut self, sent_timestamp: usize) -> Result<(), CoreError> {
        if let Some((timestamp, source_id, message)) = self.network.remove_one(sent_timestamp) {
            self.deliver(timestamp, source_id, message).await?
        }
        Ok(())
    }
}
