//! Transparent handling of processes, both native and compiled to Webassembly.

use std::ffi::OsStr;
use std::io::Error;
use std::path::{Path, PathBuf};

use crossbeam_channel::Sender;
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
    /// the handle (containing the threads/tasks that monitor the process). See [`Handle`].
    /// `None` after the process exited.
    handle: Option<Handle>,
    /// [`Sender`] to send commands (only deliver [`Message`](libproto::Message)s for now).
    command_sender: Option<Sender<ProcessCommand>>,
    /// unique id of the process in the running [`Core`](crate::core::Core).
    id: usize,
    /// Path to the executable (or Webassembly) file. Used for debugging and log printing.
    path: PathBuf,
}

impl Launcher {
    /// Creates a new [`Launcher`] ready to launch processes.
    pub fn new() -> Self {
        Self { wasm_launcher: None }
    }

    /// launches a new process from the given `path`. The process is passed the `event_sender` so that it can send [`ProcessEvent`]s to the core.
    /// It is also passed it's own unique id, so that [`ProcessEvent`]s it sends can be associated with the process (since all [`ProcessEvent`]s
    /// from all processes are sent via a single channel).
    /// If `path` points to a Webassembly file (ends in `.wasm`) a Webassembly process is started, otherwise a native process is started.
    /// TODO: support for scripting languages (i.e. Python), where launching a process needs a path and some args.
    ///
    /// Returns a handel to the launched process, or an error if launching failed (i.e. the file does not exists, or is not executable, etc.).
    pub fn launch(&mut self, path: &Path, event_sender: &Sender<ProcessEvent>, id: usize) -> Result<Process, Error> {
        let (command_sender, handle) = if path.extension() == Some(OsStr::new("wasm")) {
            self.wasm_launcher.get_or_insert_with(WasmLauncher::new)
                .launch(path, event_sender, id)
        } else {
            native::launch(path, event_sender, id)
        }?;
        Ok(Process {
            handle: Some(handle),
            command_sender: Some(command_sender),
            id,
            path: path.to_path_buf(),
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

    /// Terminate the process.
    /// This drops the `command_sender`, so that threads waiting for [`ProcessCommand`]s from the
    /// [`Core`](crate::core::Core) stop waiting and can terminate. Then waits for all threads/tasks
    /// to terminate.
    pub fn terminate(&mut self) {
        self.command_sender.take();
        if let Some(handle) = self.handle.take() {
            handle.terminate();
        }
    }

    /// Returns `true` if the process is still running
    pub fn is_running(&self) -> bool {
        self.handle.as_ref().map(|h| h.is_running()).unwrap_or(false)
    }

    /// Returns the unique id of the process.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Returns the path to the executable (or Webassembly) file from which the process was launched
    pub fn path(&self) -> &Path {
        &self.path
    }
}