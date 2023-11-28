use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use crossbeam_channel::Sender;
use crate::process::command::ProcessCommand;
use crate::process::event::ProcessEvent;

use crate::process::handle::Handle;

pub(super) fn launch(file: &Path, event_sender: &Sender<ProcessEvent>, id: usize) -> io::Result<(Sender<ProcessCommand>, Handle)> {
    log::info!("launching process {}", file.display());
    let mut child = Command::new(file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    Handle::new(id, file, event_sender, stdin, stdout, stderr, move || {
        child.wait()
            .expect("failed to wait for child process")
            .code()
            .unwrap_or(-1)
    })
}