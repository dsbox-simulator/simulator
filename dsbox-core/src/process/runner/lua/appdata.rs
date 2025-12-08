use crate::process::runner::{CommandReceiver, EventSender};
use crate::process::{ProcessCommand, ProcessEvent};
use std::time::Duration;
use tokio::sync::mpsc::error::SendError;

/// gets passed to the lua instance via [`Lua::set_app_data`] and is then available in the
/// native function implementations
pub struct DsboxData {
    /// a sender to send events to the core
    sender: EventSender,
    /// a receiver to receive commands (currently only messages) from the core
    receiver: CommandReceiver,
    /// the name of this node (useful for automatically sending log messages with extended information to the core)
    /// will be automatically extracted from a received `init` message
    own_name: Option<String>,
    /// the name of the simulation core. Used as the `dest` field for log messages.
    /// will be automatically extracted from a received `init` message
    core_name: Option<String>,
}

impl DsboxData {
    pub fn new(sender: EventSender, receiver: CommandReceiver) -> Self {
        Self {
            sender,
            receiver,
            own_name: None,
            core_name: None,
        }
    }

    pub fn own_name(&self) -> Option<&str> {
        self.own_name.as_deref()
    }

    pub fn core_name(&self) -> Option<&str> {
        self.core_name.as_deref()
    }

    pub fn into_sender(self) -> EventSender {
        self.sender
    }

    pub fn send_event(&self, event: ProcessEvent) -> Result<(), SendError<ProcessEvent>> {
        self.sender.blocking_send(event)
    }

    pub fn recv_command(&mut self, timeout: Option<Duration>) -> Option<ProcessCommand> {
        let command = if let Some(timeout) = timeout {
            // UnboundedReceiver unfortunately does not have a `block_recv_timeout` function
            tokio::runtime::Handle::current()
                .block_on(tokio::time::timeout(timeout, self.receiver.recv()))
                .unwrap_or_else(|_| None)
        } else {
            self.receiver.blocking_recv()
        };
        if let Some(ProcessCommand::Deliver(msg)) = command.as_ref() {
            if let Ok(init) = msg.payload::<libproto::init::Init>() {
                self.own_name = Some(init.name);
                self.core_name = Some(init.core_name);
            }
        }
        command
    }

    pub fn close_receiver(&mut self) {
        self.receiver.close()
    }
}
