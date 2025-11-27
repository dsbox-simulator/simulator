mod lines_helper;
mod native;
#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "lua")]
mod lua;

mod io_helper;
mod manager;
mod handle;

use crate::command::ExecutableCommand;
use crate::process::{ProcessCommand, ProcessEvent};
use std::pin::Pin;

type EventSender = tokio::sync::mpsc::Sender<ProcessEvent>;
type EventReceiver = tokio::sync::mpsc::Receiver<ProcessEvent>;
type CommandSender = tokio::sync::mpsc::UnboundedSender<ProcessCommand>;
type CommandReceiver = tokio::sync::mpsc::UnboundedReceiver<ProcessCommand>;

pub trait Runner {
    fn run(
        &mut self,
        command: ExecutableCommand,
        sender: EventSender,
        receiver: CommandReceiver,
    ) -> impl Future<Output = i32> + Send + 'static;
}

type RunnerFn = dyn FnMut(
    ExecutableCommand,
    EventSender,
    CommandReceiver,
) -> Pin<Box<dyn Future<Output = i32> + Send + 'static>>;

pub struct DynRunner {
    runner: Box<RunnerFn>,
}

impl DynRunner {
    pub fn new(mut runner: impl Runner + 'static) -> Self {
        Self {
            runner: Box::new(move |command, sender, receiver| {
                let fut = runner.run(command, sender, receiver);
                Box::pin(fut)
            }),
        }
    }

    pub fn run(
        &mut self,
        command: ExecutableCommand,
        sender: EventSender,
        receiver: CommandReceiver,
    ) -> Pin<Box<dyn Future<Output = i32> + Send + 'static>> {
        (self.runner)(command, sender, receiver)
    }
}
