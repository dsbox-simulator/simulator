use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll};

use serde::{Deserialize, Serialize};

use crate::process::{Process, ProcessCommand, ProcessEvent};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct NodeId(pub usize);

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(transparent)]
pub struct MiddlewareId(pub usize);

pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub is_client: bool,
    process_stack: Vec<Rc<RefCell<Process>>>,
    primary_index: usize,
}

impl Node {
    pub fn new(name: String, is_client: bool, process: Process) -> Self {
        Self {
            id: NodeId(0),
            name,
            is_client,
            process_stack: vec![Rc::new(RefCell::new(process))],
            primary_index: 0,
        }
    }

    pub fn commandline(&self, middleware_id: MiddlewareId) -> String {
        self.process_stack[middleware_id.0].borrow().commandline()
    }

    pub fn alias(&self, name: String) -> Self {
        Self {
            id: NodeId(0),
            name,
            is_client: self.is_client,
            process_stack: Vec::clone(&self.process_stack),
            primary_index: self.primary_index,
        }
    }

    pub fn is_same_process(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.process_stack[self.primary_index], &other.process_stack[other.primary_index])
    }

    pub fn push_middleware_before(&mut self, middleware: Process) {
        self.process_stack.insert(0, Rc::new(RefCell::new(middleware)));
        self.primary_index += 1;
    }

    pub fn push_middleware_after(&mut self, middleware: Process) {
        self.process_stack.push(Rc::new(RefCell::new(middleware)))
    }

    pub fn has_middleware(&self, middleware_id: MiddlewareId) -> bool {
        middleware_id.0 < self.process_stack.len()
    }

    pub fn send(&self, command: ProcessCommand) -> bool {
        self.send_to_middleware(command, MiddlewareId(0))
    }

    pub fn send_to_middleware(&self, command: ProcessCommand, middleware_id: MiddlewareId) -> bool {
        self.process_stack[middleware_id.0].borrow().send(command)
    }

    pub fn poll_recv_any(&self, cx: &mut Context<'_>) -> Poll<Option<(ProcessEvent, MiddlewareId)>> {
        let mut num_closed = 0;
        for (idx, process) in self.process_stack.iter().enumerate() {
            let mut borrowed = process.borrow_mut();
            let pinned = std::pin::pin!(borrowed.recv());
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

    pub fn has_finished(&self) -> bool {
        self.process_stack.iter().all(|p| p.borrow_mut().has_finished())
    }

    pub fn begin_shutdown(&mut self) {
        for process in &mut self.process_stack {
            if let Some(unique_proc) = Rc::get_mut(process) {
                unique_proc.get_mut().begin_shutdown()
            }
        }
    }

    pub async fn terminate(self) {
        for process in self.process_stack {
            if let Some(unique_proc) = Rc::into_inner(process) {
                let proc = RefCell::into_inner(unique_proc);
                proc.terminate().await
            }
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