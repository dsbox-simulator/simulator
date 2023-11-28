//! Transparent handling of processes, both native and implemented as web assembly files.

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

pub struct Launcher {
    wasm_launcher: Option<WasmLauncher>,
}

pub struct Process {
    handle: Option<Handle>,
    command_sender: Option<Sender<ProcessCommand>>,
    id: usize,
    path: PathBuf,
}

impl Launcher {
    pub fn new() -> Self {
        Self { wasm_launcher: None }
    }
    pub fn launch(&mut self, file: &Path, event_sender: &Sender<ProcessEvent>, id: usize) -> Result<Process, Error> {
        let (command_sender, handle) = if file.extension() == Some(OsStr::new("wasm")) {
            self.wasm_launcher.get_or_insert_with(WasmLauncher::new)
                .launch(file, event_sender, id)
        } else {
            native::launch(file, event_sender, id)
        }?;
        Ok(Process {
            handle: Some(handle),
            command_sender: Some(command_sender),
            id,
            path: file.to_path_buf(),
        })
    }
}

impl Process {
    pub fn send(&mut self, value: ProcessCommand) -> bool {
        if let Some(sender) = &mut self.command_sender {
            sender.send(value).is_ok()
        } else { false }
    }

    pub fn terminate(&mut self) {
        self.command_sender.take();
        if let Some(handle) = self.handle.take() {
            handle.terminate();
        }
    }

    pub fn is_running(&self) -> bool {
        self.handle.as_ref().map(|h| h.is_running()).unwrap_or(false)
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}