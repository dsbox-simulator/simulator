use std::fmt::{Display, Formatter};
use std::future::Future;
use std::task::{Context, Poll};

use serde::{Deserialize, Serialize};

use crate::process::{Process, ProcessCommand, ProcessEvent};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct NodeId(pub usize);

pub struct Node {
    /// the [`NodeId`] of this node
    pub id: NodeId,
    /// the name of this node
    pub name: String,
    /// `true` if this is the test node
    pub is_test: bool,
    /// `true` if the test node has requested to receive an `exited` message upon this nodes termination
    pub exited_message_requested: bool,
    /// `true` if this node is still required to send a `register` message to the core. Used for better error messages
    pub requires_registration: bool,
    /// handle to the process that "runs" this node
    process: Process,
}

impl Node {
    pub fn new(name: String, is_test: bool, exited_message_requested: bool, requires_registration: bool, process: Process) -> Self {
        Self {
            id: NodeId(0),
            name,
            is_test,
            requires_registration,
            exited_message_requested,
            process,
        }
    }

    pub fn commandline(&self) -> String {
        self.process.commandline()
    }

    pub fn send(&self, command: ProcessCommand) -> bool {
        self.process.send(command)
    }

    pub fn poll_recv(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<ProcessEvent>> {
        std::pin::pin!(self.process.recv()).poll(cx)
    }

    pub fn has_finished(&mut self) -> bool {
        self.process.has_finished()
    }

    pub fn begin_shutdown(&mut self) {
        self.process.begin_shutdown()
    }

    pub async fn terminate(self) {
        self.process.terminate().await
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
