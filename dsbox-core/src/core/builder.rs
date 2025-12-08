use crate::capabilities::Capability;
use crate::command::RunnerCommand;
use crate::core::{Core, InitialLaunch};
use crate::process::{Runner, RunnerManger};
use enumflags2::BitFlags;
use std::collections::HashMap;

const DEFAULT_CORE_NAME: &'static str = "core";

/// A builder for a [`Core`].
pub struct Builder {
    pub(super) runner_manager: RunnerManger,
    pub(super) commands: HashMap<String, (RunnerCommand, BitFlags<Capability>)>,
    pub(super) launch_initial: Vec<InitialLaunch>,
    pub(super) interactive: bool,
    pub(super) core_name: String,
}

impl Builder {
    /// Create a new builder with the specified commands for the test and server nodes
    pub(super) fn new() -> Self {
        Self {
            commands: HashMap::new(),
            launch_initial: Vec::new(),
            interactive: false,
            core_name: DEFAULT_CORE_NAME.to_string(),
            runner_manager: RunnerManger::new(),
        }
    }

    /// register a command that can be launched
    pub fn register_command(
        mut self,
        name: impl Into<String>,
        command: RunnerCommand,
        capabilities: BitFlags<Capability>,
    ) -> Self {
        self.commands.insert(name.into(), (command, capabilities));
        self
    }

    /// when starting to run the core, launch a command initially
    pub fn launch(
        mut self,
        command: impl Into<String>,
        name: impl Into<String>,
        requires_registration: bool,
    ) -> Self {
        self.launch_initial.push(InitialLaunch {
            command: command.into(),
            name: name.into(),
            requires_registration,
            weak: false,
        });
        self
    }

    /// when starting to run the core, launch a command initially. The resulting node
    /// will not keep the core running, if it is the only node remaining
    pub fn launch_weak(
        mut self,
        command: impl Into<String>,
        name: impl Into<String>,
        requires_registration: bool,
    ) -> Self {
        self.launch_initial.push(InitialLaunch {
            command: command.into(),
            name: name.into(),
            requires_registration,
            weak: true,
        });
        self
    }

    /// enable/disable interactive mode
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// override the name of the simulation core (default: `"core"`)
    pub fn core_name(mut self, core_name: impl Into<String>) -> Self {
        self.core_name = core_name.into();
        self
    }

    /// register a new runner (e.g. for native processes, or the built-in lua interpreter)
    pub fn register_runner(
        mut self,
        name: impl Into<String>,
        runner: impl Runner + Send + Sync + 'static,
    ) -> Self {
        self.runner_manager.register_runner(name.into(), runner);
        self
    }

    /// finish building and create a [`Core`]
    pub fn build(self) -> Core {
        Core::from(self)
    }
}
