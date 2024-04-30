use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::process::{ProcessCommand, ProcessEvent};
use crate::process::io_helper::process_io_helper;

/// launches a new native process with the given `path` to an executable and creates a new [`Handle`]
/// from the childs `stdin`, `stdout` and `stderr`.
/// Returns the [`Handle`] and a [`Sender`] that can be used to send [`ProcessCommand`]s to the process.
pub(super) fn launch(path: &Path, args: &[String], command_receiver: UnboundedReceiver<ProcessCommand>, event_sender: Sender<ProcessEvent>) -> tokio::io::Result<(JoinHandle<()>, oneshot::Receiver<()>)> {
    log::trace!("launching process `{}`, args: {args:?}", path.display());
    let mut child = Command::new(path)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let (finished_tx, finished_rx) = oneshot::channel();
    Ok((tokio::task::spawn(async move {
        process_io_helper(event_sender, command_receiver, stdin, stdout, stderr, async move {
            match child.wait().await {
                Ok(status) => status.code().unwrap_or(0),
                Err(_) => -1,
            }
        }, finished_tx).await
    }), finished_rx))
}