use std::fmt::{Display, Formatter};
use std::future::Future;
use std::task::{Context, Poll};

use serde::{Deserialize, Serialize};

use crate::process::{Process, ProcessCommand, ProcessEvent};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct NodeId(pub usize);

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(transparent)]
pub struct MiddlewareId(pub usize);

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
    /// that stack of processes that implement this node. This is where Middlewares are implemented, though support is experimental and un-documented
    process_stack: Vec<Process>,
    /// the index in `process_stack` where the actual implementation of this process resides (as opposed to middleware processes)
    primary_index: usize,
}

impl Node {
    pub fn new(name: String, is_test: bool, exited_message_requested: bool, requires_registration: bool, process: Process) -> Self {
        Self {
            id: NodeId(0),
            name,
            is_test,
            requires_registration,
            exited_message_requested,
            process_stack: vec![process],
            primary_index: 0,
        }
    }

    pub fn commandline(&self, middleware_id: MiddlewareId) -> String {
        self.process_stack[middleware_id.0].commandline()
    }

    pub fn push_middleware_before(&mut self, middleware: Process) {
        self.process_stack.insert(0, middleware);
        self.primary_index += 1;
    }

    pub fn push_middleware_after(&mut self, middleware: Process) {
        self.process_stack.push(middleware)
    }

    pub fn has_middleware(&self, middleware_id: MiddlewareId) -> bool {
        middleware_id.0 < self.process_stack.len()
    }

    pub fn send(&self, command: ProcessCommand) -> bool {
        self.send_to_middleware(command, MiddlewareId(0))
    }

    pub fn send_to_middleware(&self, command: ProcessCommand, middleware_id: MiddlewareId) -> bool {
        self.process_stack[middleware_id.0].send(command)
    }

    pub fn poll_recv_any(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<(ProcessEvent, MiddlewareId)>> {
        let mut num_closed = 0;
        for (idx, process) in self.process_stack.iter_mut().enumerate() {
            let pinned = std::pin::pin!(process.recv());
            match pinned.poll(cx) {
                Poll::Ready(Some(event)) => return Poll::Ready(Some((event, MiddlewareId(idx)))),
                Poll::Ready(None) => num_closed += 1,
                _ => {}
            }
        }
        if num_closed < self.process_stack.len() {
            Poll::Pending
        } else {
            Poll::Ready(None)
        }
    }

    pub fn has_finished(&mut self) -> bool {
        self.process_stack.iter_mut().all(|p| p.has_finished())
    }

    pub fn begin_shutdown(&mut self) {
        for process in &mut self.process_stack {
            process.begin_shutdown()
        }
    }

    pub async fn terminate(self) {
        for process in self.process_stack {
            process.terminate().await
        }
    }
}

impl MiddlewareId {
    pub fn is_top(self) -> bool {
        self.0 == 0
    }

    pub fn above(self) -> Self {
        Self(self.0 - 1)
    }

    pub fn below(self) -> Self {
        Self(self.0 + 1)
    }
}

impl Display for MiddlewareId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
