//! Transparent handling of processes, both native, compiled to Webassembly and in the form of a lua script.

use std::ffi::OsStr;
use std::io::{Error, ErrorKind};
use std::path::Path;

use tokio::sync::mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;
use tokio::task::JoinHandle;

use crate::cli::Args;
pub use crate::process::command::ProcessCommand;
pub use crate::process::event::ProcessEvent;
use crate::process::lua::LuaLauncher;
#[cfg(feature = "wasm")]
use crate::process::wasm::WasmLauncher;

mod native;
#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "lua")]
mod lua;
mod event;
mod command;
mod io_helper;

/// Used to launch new processes. No state is necessary for native processes or lua scripts,
/// but for Webassembly it is useful to have some persistent state between launching processes.
pub struct Launcher {
    /// Some state used for launching Webassembly processes, initialized as needed.
    #[cfg(feature = "wasm")]
    wasm_launcher: Option<WasmLauncher>,
    #[cfg(feature = "lua")]
    allow_lua_unsafe: bool,
    #[cfg(feature = "lua")]
    lua_launcher: Option<LuaLauncher>,
}

/// Handle to a running process
pub struct Process {
    sender: Option<UnboundedSender<ProcessCommand>>,
    receiver: Receiver<ProcessEvent>,
    join_handle: JoinHandle<()>,
    finished: oneshot::Receiver<()>,
    pub path: String,
    pub args: Vec<String>,
}

impl Launcher {
    /// Creates a new [`Launcher`] ready to launch processes.
    pub fn new(#[allow(unused)]args: &Args) -> Self {
        Self {
            #[cfg(feature = "wasm")]
            wasm_launcher: None,
            #[cfg(feature = "lua")]
            allow_lua_unsafe: args.lua_unsafe,
            #[cfg(feature = "lua")]
            lua_launcher: None,
        }
    }

    /// launches a new process from the given `path`. The process is passed the `event_sender` so that it can send [`ProcessEvent`]s to the core.
    /// It is also passed its own unique id, so that [`ProcessEvent`]s it sends can be associated with the process (since all [`ProcessEvent`]s
    /// from all processes are sent via a single channel).
    /// If `path` points to a Webassembly file (ends in `.wasm`) a Webassembly process is started,
    /// if it points to a lua script (ends in `.lua`) that script is loaded and started,
    /// otherwise a native process is started.
    ///
    /// Returns a handel to the launched process, or an error if launching failed (i.e. the file does not exist, or is not executable, etc.).
    pub async fn launch(&mut self, command: &str, for_client: bool) -> Result<Process, Error> {
        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(1);
        let Some(mut args) = shlex::split(command) else {
            return Err(Error::new(ErrorKind::InvalidInput, format!("failed to parse command string: {command:?}")));
        };
        let path = args.remove(0);
        let executable = Path::new(&path);
        let ext = executable.extension();
        let (join_handle, finished) = if ext == Some(OsStr::new("wasm")) {
            self.launch_wasm(executable, &args, command_receiver, event_sender).await?
        } else if ext == Some(OsStr::new("lua")) {
            self.launch_lua(executable, &args, for_client, command_receiver, event_sender).await?
        } else {
            native::launch(executable, &args, command_receiver, event_sender)?
        };

        Ok(Process {
            sender: Some(command_sender),
            receiver: event_receiver,
            join_handle,
            finished,
            path,
            args,
        })
    }

    #[cfg(feature = "wasm")]
    async fn launch_wasm(&mut self, path: &Path, args: &[String], command_receiver: UnboundedReceiver<ProcessCommand>, event_sender: Sender<ProcessEvent>) -> tokio::io::Result<(JoinHandle<()>, oneshot::Receiver<()>)> {
        self.wasm_launcher.get_or_insert_with(WasmLauncher::new)
            .launch(path, args, command_receiver, event_sender).await
    }
    #[cfg(not(feature = "wasm"))]
    async fn launch_wasm(&mut self, _: &Path, _: &[String], _: UnboundedReceiver<ProcessCommand>, _: Sender<ProcessEvent>) -> tokio::io::Result<(JoinHandle<()>, oneshot::Receiver<()>)> {
        panic!("this version of dsbox was built without wasm support")
    }

    #[cfg(feature = "lua")]
    async fn launch_lua(&mut self, path: &Path, args: &[String], for_client: bool, command_receiver: UnboundedReceiver<ProcessCommand>, event_sender: Sender<ProcessEvent>) -> tokio::io::Result<(JoinHandle<()>, oneshot::Receiver<()>)> {
        if self.lua_launcher.is_none() {
            self.lua_launcher = Some(LuaLauncher::new().await)
        }
        self.lua_launcher.as_mut().unwrap()
            .launch(path, args.to_vec(), self.allow_lua_unsafe && for_client, command_receiver, event_sender)
    }
    #[cfg(not(feature = "lua"))]
    async fn launch_lua(&mut self, _: &Path, _: &[String], _: bool, _: UnboundedReceiver<ProcessCommand>, _: Sender<ProcessEvent>) -> tokio::io::Result<(JoinHandle<()>, oneshot::Receiver<()>)> {
        panic!("this version of dsbox was built without lua support")
    }
}

impl Process {
    /// Send a [`ProcessCommand`] to the process.
    pub fn send(&self, value: ProcessCommand) -> bool {
        if let Some(sender) = &self.sender {
            sender.send(value).is_ok()
        } else { false }
    }

    pub async fn recv(&mut self) -> Option<ProcessEvent> {
        self.receiver.recv().await
    }

    /// This drops the `command_sender`, so that threads waiting for [`ProcessCommand`]s from the
    /// [`Core`](crate::core::Core) stop waiting and terminate.
    pub fn begin_shutdown(&mut self) {
        if self.sender.is_some() {
            log::trace!("begin shutdown of process `{}`", self.commandline());
        }
        self.sender.take();
    }

    /// This waits for the tasks that handle the processes IO to finish
    pub async fn terminate(mut self) {
        self.begin_shutdown();
        self.receiver.close();
        self.join_handle.await.ok();
    }

    /// Returns `true` if the process has stopped running and all messages have been received
    pub fn has_finished(&mut self) -> bool {
        if !self.receiver.is_empty() { return false; }
        match self.finished.try_recv() {
            Err(TryRecvError::Empty) => false,
            _ => true,
        }
    }

    /// returns the full commandline (path + args) that was used to launch the process
    pub fn commandline(&self) -> String {
        let parts = std::iter::once(self.path.as_str()).chain(self.args.iter().map(|s| s.as_str()));
        shlex::try_join(parts).unwrap()
    }
}

