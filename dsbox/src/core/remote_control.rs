//! Commands used to control the execution of the simulation

use crate::core::{Core, CoreState};
use crate::core::error::CoreError;

/// A command for the [`Core`] to control its execution
#[cfg_attr(not(feature = "webapp"), allow(unused))]
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
}

impl Core {
    /// handles a single [`RemoteCommand`]
    pub async fn handle_command(&mut self, command: RemoteCommand) -> Result<(), CoreError> {
        match command {
            RemoteCommand::Break => self.set_state(CoreState::Paused),
            RemoteCommand::Step => self.set_state(CoreState::Stepping),
            RemoteCommand::Resume => self.set_state(CoreState::Running),
            RemoteCommand::Deliver(sent_timestamp) => self.deliver_by_timestamp(sent_timestamp).await?,
            RemoteCommand::Drop(sent_timestamp) => self.drop_by_timestamp(sent_timestamp),
        }
        Ok(())
    }

    fn set_state(&mut self, state: CoreState) {
        self.state = state;
    }

    fn drop_by_timestamp(&mut self, sent_timestamp: usize) {
        self.network.remove_one(sent_timestamp);
    }

    async fn deliver_by_timestamp(&mut self, sent_timestamp: usize) -> Result<(), CoreError> {
        if let Some((timestamp, message)) = self.network.remove_one(sent_timestamp) {
            self.deliver(timestamp, message).await?
        }
        Ok(())
    }
}