use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use crossbeam_channel::Sender;
use crate::process::command::ProcessCommand;
use crate::process::event::ProcessEvent;

use crate::process::handle::Handle;

/// launches a new native process with the given `path` to an executable and creates a new [`Handle`]
/// from the childs `stdin`, `stdout` and `stderr`.
/// Returns the [`Handle`] and a [`Sender`] that can be used to send [`ProcessCommand`]s to the process.
pub(super) fn launch(path: &Path, event_sender: &Sender<ProcessEvent>, id: usize) -> io::Result<(Sender<ProcessCommand>, Handle)> {
    log::info!("launching process {}", path.display());
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    Handle::new(id, path, event_sender, stdin, stdout, stderr, move || {
        child.wait()
            .expect("failed to wait for child process")
            .code()
            .unwrap_or(-1)
    })
}