//! Transparent handling of processes, both native, compiled to Webassembly and in the form of a lua script.

use std::ffi::OsStr;
use std::io::{Error, ErrorKind};
use std::path::Path;

use tokio::sync::mpsc::{Sender, UnboundedSender};

pub use crate::process::command::ProcessCommand;
pub use crate::process::event::{ProcessEvent, ProcessEventKind};
use crate::process::handle::Handle;
#[cfg(feature = "wasm")]
use crate::process::wasm::WasmLauncher;

mod native;
#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "lua")]
mod lua;
mod handle;
mod event;
mod command;

/// Used to launch new processes. No state is necessary for native processes or lua scripts,
/// but for Webassembly it is useful to have some persistent state between launching processes.
pub struct Launcher {
    /// Some state used for launching Webassembly processes, initialized as needed.
    #[cfg(feature = "wasm")]
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
        Self { #[cfg(feature = "wasm")]wasm_launcher: None }
    }

    /// launches a new process from the given `path`. The process is passed the `event_sender` so that it can send [`ProcessEvent`]s to the core.
    /// It is also passed its own unique id, so that [`ProcessEvent`]s it sends can be associated with the process (since all [`ProcessEvent`]s
    /// from all processes are sent via a single channel).
    /// If `path` points to a Webassembly file (ends in `.wasm`) a Webassembly process is started,
    /// if it points to a lua script (ends in `.lua`) that script is loaded and started,
    /// otherwise a native process is started.
    ///
    /// Returns a handel to the launched process, or an error if launching failed (i.e. the file does not exist, or is not executable, etc.).
    pub async fn launch(&mut self, command: &str, event_sender: &Sender<ProcessEvent>, id: usize, name: String) -> Result<Process, Error> {
        let Some(mut args) = shlex::split(command) else {
            return Err(Error::new(ErrorKind::InvalidInput, format!("failed to parse command string: {command:?}")));
        };
        let path = args.remove(0);
        let executable = Path::new(&path);
        let ext = executable.extension();
        let (command_sender, handle) = if ext == Some(OsStr::new("wasm")) {
            self.launch_wasm(executable, &args, event_sender, id).await
        } else if ext == Some(OsStr::new("lua")) {
            self.launch_lua(executable, &args, event_sender, id).await
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

    #[cfg(feature = "wasm")]
    async fn launch_wasm(&mut self, executable: &Path, args: &[String], event_sender: &Sender<ProcessEvent>, id: usize) -> Result<(UnboundedSender<ProcessCommand>, Handle), Error> {
        self.wasm_launcher.get_or_insert_with(WasmLauncher::new)
            .launch(executable, &args, event_sender, id).await
    }
    #[cfg(not(feature = "wasm"))]
    async fn launch_wasm(&mut self, _: &Path, _: &[String], _: &Sender<ProcessEvent>, _: usize) -> Result<(UnboundedSender<ProcessCommand>, Handle), Error> {
        panic!("this version of dsbox was built without wasm support")
    }

    #[cfg(feature = "lua")]
    async fn launch_lua(&mut self, executable: &Path, args: &[String], event_sender: &Sender<ProcessEvent>, id: usize) -> Result<(UnboundedSender<ProcessCommand>, Handle), Error> {
        lua::launch(executable, args, event_sender, id)
    }
    #[cfg(not(feature = "lua"))]
    async fn launch_lua(&mut self, _: &Path, _: &[String], _: &Sender<ProcessEvent>, _: usize) -> Result<(UnboundedSender<ProcessCommand>, Handle), Error> {
        panic!("this version of dsbox was built without lua support")
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