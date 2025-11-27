use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::task::JoinHandle;

use crate::process::launcher::io_helper::process_io_helper;
use crate::process::{ProcessCommand, ProcessEvent};

/// An extension for [`Command`] that disables the creation of a terminal window for child
/// processes under windows. Implemented below, does nothing outside of windows
trait CommandExt {
    fn disable_terminal_window(&mut self) -> &mut Self;
}

/// launches a new native process with the given `path` to an executable and the sender and receiver for commands/events
pub(in crate::process) fn launch(
    path: &Path,
    args: &[String],
    command_receiver: UnboundedReceiver<ProcessCommand>,
    event_sender: Sender<ProcessEvent>,
) -> tokio::io::Result<JoinHandle<()>> {
    log::trace!("launching process `{}`, args: {args:?}", path.display());
    let mut child = Command::new(path)
        .args(args)
        .disable_terminal_window()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    Ok(tokio::task::spawn(async move {
        process_io_helper(
            event_sender,
            command_receiver,
            stdin,
            stdout,
            stderr,
            async move {
                match child.wait().await {
                    Ok(status) => status.code().unwrap_or(0),
                    Err(_) => -1,
                }
            },
        )
        .await
    }))
}

impl CommandExt for Command {
    #[cfg(target_os = "windows")]
    fn disable_terminal_window(&mut self) -> &mut Self {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        self.creation_flags(CREATE_NO_WINDOW)
    }

    #[cfg(not(target_os = "windows"))]
    fn disable_terminal_window(&mut self) -> &mut Self {
        self
    }
}
