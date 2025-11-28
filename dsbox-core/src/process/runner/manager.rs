use crate::command::RunnerCommand;
use crate::process::runner::handle::RunningHandle;
use crate::process::runner::{DynRunner, Runner};
use std::collections::HashMap;
use std::fmt::Formatter;

pub struct RunnerManger {
    registered_runners: HashMap<String, DynRunner>,
}

#[derive(Copy, Clone, Debug)]
pub struct UnknownRunner;

impl RunnerManger {
    pub fn new() -> Self {
        Self {
            registered_runners: HashMap::new(),
        }
    }

    pub fn register_runner(&mut self, name: String, runner: impl Runner + 'static) {
        self.registered_runners.insert(name, DynRunner::new(runner));
    }

    pub fn available_runners(&mut self) -> impl Iterator<Item = &String> {
        self.registered_runners.keys()
    }

    pub fn run(
        &mut self,
        command: &RunnerCommand,
    ) -> Result<RunningHandle, UnknownRunner> {
        let runner = self
            .registered_runners
            .get_mut(command.runner())
            .ok_or_else(|| UnknownRunner)?;
        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(1);
        let handle =
            tokio::task::spawn(runner.run(command.args(), event_sender, command_receiver));
        Ok(RunningHandle::new(
            command_sender,
            event_receiver,
            handle,
            command.args().join(" "),
        ))
    }
}

impl std::fmt::Display for UnknownRunner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("unknown runner")
    }
}

impl std::error::Error for UnknownRunner {}
