use std::cell::RefCell;
use std::rc::Rc;

use tokio::select;

use crate::process::{Process, ProcessCommand, ProcessEvent};

pub struct Node {
    pub id: usize,
    pub name: String,
    pub is_client: bool,
    process: Rc<RefCell<Process>>,
    proxy: Option<Rc<RefCell<Process>>>,
}

impl Node {
    pub fn new(name: String, is_client: bool, process: Process) -> Self {
        Self {
            id: 0,
            name,
            is_client,
            process: Rc::new(RefCell::new(process)),
            proxy: None,
        }
    }

    pub fn commandline(&self) -> String {
        self.process.borrow().commandline()
    }

    pub fn alias(&self, name: String) -> Self {
        Self {
            id: 0,
            name,
            is_client: self.is_client,
            process: Rc::clone(&self.process),
            proxy: self.proxy.clone(),
        }
    }

    pub fn is_same_process(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.process, &other.process)
    }

    pub fn set_proxy(&mut self, proxy: Option<Process>) {
        self.proxy = proxy.map(RefCell::new).map(Rc::new);
    }
    pub fn has_proxy(&self) -> bool {
        self.proxy.is_some()
    }

    pub fn send(&self, command: ProcessCommand) -> (bool, bool) {
        if let Some(proxy) = &self.proxy {
            (proxy.borrow().send(command), true)
        } else {
            (self.process.borrow().send(command), false)
        }
    }

    pub async fn recv(&self) -> (Option<ProcessEvent>, bool) {
        if let Some(proxy) = &self.proxy {
            let (mut proxy, mut node) = (proxy.borrow_mut(), self.process.borrow_mut());
            select! {
                from_proxy = proxy.recv() => {
                    (from_proxy, true)
                }
                from_node = node.recv() => {
                    (from_node, false)
                }
            }
        } else {
            (self.process.borrow_mut().recv().await, false)
        }
    }

    pub fn send_ignore_proxy(&self, command: ProcessCommand) -> bool {
        self.process.borrow().send(command)
    }

    pub fn send_proxy(&self, command: ProcessCommand) -> bool {
        self.proxy.as_ref().unwrap().borrow().send(command)
    }

    pub async fn recv_ignore_proxy(&self) -> Option<ProcessEvent> {
        self.process.borrow_mut().recv().await
    }

    pub fn has_finished(&self) -> bool {
        self.process.borrow_mut().has_finished()
    }

    pub fn begin_shutdown(&mut self) {
        if let Some(unique_proc) = Rc::get_mut(&mut self.process) {
            unique_proc.get_mut().begin_shutdown()
        }
        if let Some(unique_proxy) = self.proxy.as_mut().and_then(Rc::get_mut) {
            unique_proxy.get_mut().begin_shutdown();
        }
    }

    pub async fn terminate(self) {
        if let Some(unique_proc) = Rc::into_inner(self.process) {
            let proc = RefCell::into_inner(unique_proc);
            proc.terminate().await
        }
        if let Some(unique_proxy) = self.proxy.and_then(Rc::into_inner) {
            let proc = RefCell::into_inner(unique_proxy);
            proc.terminate().await
        }
    }
}