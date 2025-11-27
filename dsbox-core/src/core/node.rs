use crate::capabilities;
use crate::capabilities::Capability;
use crate::process::{Process, ProcessCommand, ProcessEvent};
use enumflags2::BitFlags;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct NodeId(usize);

pub struct Node {
    /// the [`NodeId`] of this node
    pub id: NodeId,
    /// if this node was launched by another node sending a [`Launch`](libproto::system::Launch) Message
    /// that nodes id will be set here. This node will then be shut down, if the launching node shuts down
    pub launched_by: Option<NodeId>,
    /// the name of this node
    pub name: String,
    /// the capabilities (to send system messages) of this node
    capabilities: BitFlags<Capability>,
    /// `true` if the test node has requested to receive an `exited` message upon this nodes termination
    pub exited_message_requested: bool,
    /// `true` if this node is still required to send a `register` message to the core. Used for better error messages
    pub requires_registration: bool,
    /// handle to the process that "runs" this node
    process: Process,
}

impl Node {
    pub fn new(
        name: String,
        launched_by: Option<NodeId>,
        capabilities: BitFlags<Capability>,
        exited_message_requested: bool,
        requires_registration: bool,
        process: Process,
    ) -> Self {
        Self {
            id: NodeId::next(),
            launched_by,
            name,
            capabilities,
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

    pub fn has_capability(&self, message_type: impl AsRef<str>) -> bool {
        capabilities::has_capability(self.capabilities, message_type)
    }

    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<ProcessEvent>> {
        std::pin::pin!(self.process.recv()).poll(cx)
    }

    pub fn has_finished(&self) -> bool {
        self.process.has_finished()
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.process.exit_code()
    }

    pub fn begin_shutdown(&mut self) {
        self.process.begin_shutdown()
    }

    pub async fn terminate(&mut self) {
        self.process.terminate().await
    }
}

static NEXT_NODE_ID: AtomicUsize = AtomicUsize::new(1);
impl NodeId {
    /// node ids start at one, for whatever reason

    pub fn next() -> Self {
        Self(NEXT_NODE_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
