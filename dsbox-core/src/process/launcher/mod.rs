use std::ffi::OsStr;
use std::io::Error;
use std::path::Path;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::task::JoinHandle;
use crate::process::{Process, ProcessCommand, ProcessEvent};
use lua::LuaLauncher;
use wasm::WasmLauncher;

mod native;
#[cfg(feature = "wasm")]
pub mod wasm;
#[cfg(feature = "lua")]
pub mod lua;
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

impl Launcher {
    /// Creates a new [`Launcher`] ready to launch processes.
    pub fn new(#[allow(unused)] allow_lua_unsafe: bool) -> Self {
        Self {
            #[cfg(feature = "wasm")]
            wasm_launcher: None,
            #[cfg(feature = "lua")]
            allow_lua_unsafe,
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
    pub async fn launch(
        &mut self,
        command: crate::command::ExecutableCommand,
        name: String,
        core_name: String,
    ) -> Result<Process, Error> {
        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::channel(1);
        let executable = Path::new(&command.program);
        let ext = executable.extension();
        let join_handle = if ext == Some(OsStr::new("wasm")) {
            self.launch_wasm(executable, &command.args, command_receiver, event_sender)
                .await?
        } else if ext == Some(OsStr::new("lua")) {
            self.launch_lua(
                executable,
                &command.args,
                command_receiver,
                event_sender,
                name,
                core_name,
            )
            .await?
        } else {
            native::launch(executable, &command.args, command_receiver, event_sender)?
        };

        Ok(Process {
            sender: Some(command_sender),
            receiver: event_receiver,
            join_handle: Some(join_handle),
            exit_code: None,
            command,
        })
    }

    #[cfg(feature = "wasm")]
    async fn launch_wasm(
        &mut self,
        path: &Path,
        args: &[String],
        command_receiver: UnboundedReceiver<ProcessCommand>,
        event_sender: Sender<ProcessEvent>,
    ) -> tokio::io::Result<JoinHandle<()>> {
        self.wasm_launcher
            .get_or_insert_with(WasmLauncher::new)
            .launch(path, args, command_receiver, event_sender)
            .await
    }
    #[cfg(not(feature = "wasm"))]
    async fn launch_wasm(
        &mut self,
        _: &Path,
        _: &[String],
        _: UnboundedReceiver<ProcessCommand>,
        _: Sender<ProcessEvent>,
    ) -> tokio::io::Result<JoinHandle<()>> {
        panic!("this version of dsbox was built without wasm support")
    }

    #[cfg(feature = "lua")]
    async fn launch_lua(
        &mut self,
        path: &Path,
        args: &[String],
        command_receiver: UnboundedReceiver<ProcessCommand>,
        event_sender: Sender<ProcessEvent>,
        name: String,
        core_name: String,
    ) -> tokio::io::Result<JoinHandle<()>> {
        if self.lua_launcher.is_none() {
            self.lua_launcher = Some(LuaLauncher::new().await)
        }
        self.lua_launcher.as_mut().unwrap().launch(
            path,
            args.to_vec(),
            self.allow_lua_unsafe,
            command_receiver,
            event_sender,
            name,
            core_name,
        )
    }
    #[cfg(not(feature = "lua"))]
    async fn launch_lua(
        &mut self,
        _: &Path,
        _: &[String],
        _: bool,
        _: UnboundedReceiver<ProcessCommand>,
        _: Sender<ProcessEvent>,
        _: String,
        _: String,
    ) -> tokio::io::Result<JoinHandle<()>> {
        panic!("this version of dsbox was built without lua support")
    }
}