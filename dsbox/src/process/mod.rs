//! Transparent handling of processes, both native and compiled to Webassembly.

use std::ffi::OsStr;
use std::io::{Error, ErrorKind};
use std::path::Path;

use tokio::sync::mpsc::{Sender, UnboundedSender};

pub use crate::process::command::ProcessCommand;
pub use crate::process::event::{ProcessEvent, ProcessEventKind};
use crate::process::handle::Handle;
use crate::process::wasm::WasmLauncher;

mod native;
mod wasm;
mod handle;
mod event;
mod command;

/// Used to launch new processes. No state is necessary for native processes,
/// but for Webassembly it is useful to have some persistent state between launching processes.
pub struct Launcher {
    /// Some state used for launching Webassembly processes, initialized as needed.
    wasm_launcher: Option<WasmLauncher>,
}

/// Handle to a running process
pub struct Process {
    /// the handle (containing the tasks that monitor the process). See [`Handle`].
    handle: Handle,
    /// [`Sender`] to send commands (only deliver [`Message`](libproto::Message)s for now).
    command_sender: Option<UnboundedSender<ProcessCommand>>,
    /// unique id of the process in the running [`Core`](crate::core::Core).
    id: usize,

    /// a name of the process for easier identification in the logs (typically "client" or "server")
    name: String,

    /// Path to the executable (or Webassembly) file. Used for debugging and log printing.
    path: String,

    /// args that were passed when launching the process
    args: Vec<String>,
}

impl Launcher {
    /// Creates a new [`Launcher`] ready to launch processes.
    pub fn new() -> Self {
        Self { wasm_launcher: None }
    }

    /// launches a new process from the given `path`. The process is passed the `event_sender` so that it can send [`ProcessEvent`]s to the core.
    /// It is also passed its own unique id, so that [`ProcessEvent`]s it sends can be associated with the process (since all [`ProcessEvent`]s
    /// from all processes are sent via a single channel).
    /// If `path` points to a Webassembly file (ends in `.wasm`) a Webassembly process is started, otherwise a native process is started.
    ///
    /// Returns a handel to the launched process, or an error if launching failed (i.e. the file does not exist, or is not executable, etc.).
    pub async fn launch(&mut self, command: &str, event_sender: &Sender<ProcessEvent>, id: usize, name: String) -> Result<Process, Error> {
        let Some(mut args) = shlex::split(command) else {
            return Err(Error::new(ErrorKind::InvalidInput, format!("failed to parse command string: {command:?}")));
        };
        let path = args.remove(0);
        let executable = Path::new(&path);

        let (command_sender, handle) = if executable.extension() == Some(OsStr::new("wasm")) {
            self.wasm_launcher.get_or_insert_with(WasmLauncher::new)
                .launch(executable, &args, event_sender, id).await
        } else {
            native::launch(executable, &args, event_sender, id)
        }?;
        Ok(Process {
            handle,
            command_sender: Some(command_sender),
            id,
            name,
            path,
            args,
        })
    }
}

impl Process {
    /// Send a [`ProcessCommand`] to the process.
    pub fn send(&mut self, value: ProcessCommand) -> bool {
        if let Some(sender) = &mut self.command_sender {
            sender.send(value).is_ok()
        } else { false }
    }

    /// This drops the `command_sender`, so that threads waiting for [`ProcessCommand`]s from the
    /// [`Core`](crate::core::Core) stop waiting and terminate.
    pub fn begin_shutdown(&mut self) {
        if self.is_running() {
            log::trace!("shutting down node {} (process {})", self.id(), self.path());
        }
        self.command_sender.take();
    }

    /// This waits for the tasks that handle the processes IO to finish
    pub async fn terminate(mut self) {
        self.begin_shutdown();
        self.handle.terminate().await
    }

    /// Returns `true` if the process is still running
    pub fn is_running(&self) -> bool {
        self.handle.is_running()
    }

    /// Returns the unique id of the process.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Returns the unique id of the process (typically "client" or "server").
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the path to the executable (or Webassembly) file from which the process was launched
    pub fn path(&self) -> &str {
        &self.path
    }

    /// returns the arguments that were passed to the executable (or Webassembly module)
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// returns the full commandline (path + args) that was used to launch the process
    pub fn commandline(&self) -> String {
        let parts = std::iter::once(self.path()).chain(self.args.iter().map(|s| s.as_str()));
        shlex::try_join(parts).unwrap()
    }
}