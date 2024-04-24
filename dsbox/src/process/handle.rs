//! A Handle on the tasks that monitor a process.
//!
//! For each running process, four tasks are spawned. Each one has a different function.
//! - one waits for [`ProcessCommand`]s from the [`Core`](crate::core::Core) and delivers them to the process
//! - one waits for lines one the processes `stdout`, attempts to deserialize them into [`Message`]s
//!   and sends them to the [`Core`](crate::core::Core).
//! - one waits for lines on the processes `stderr` and sends them to the [`Core`](crate::core::Core) as log lines.
//! - one just waits for the process to exit, and sends a notification to the [`Core`](crate::core::Core)

use std::future::Future;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::mpsc::{Sender, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use libproto::Message;

use crate::process::command::ProcessCommand;
use crate::process::event::{ProcessEvent, ProcessEventKind};

/// A handle to all four tasks of a process.
pub struct Handle {
    /// handle to the tasks that reads [`Message`]s from the process.d
    reader: JoinHandle<()>,
    /// handle to the task that writes [`Message`]s to the process.
    writer: JoinHandle<()>,
    /// handle to the task that reads log lines.
    log: JoinHandle<()>,
    /// handle to the task that waits for the process to exit.
    child: JoinHandle<()>,
}

impl Handle {
    /// Creates a new [`Handle`]
    /// `stdin`, `stdout` and `stderr` are generics, because they are of a different type for native
    /// and Webassembly processes. Since they are only passed off to different tasks, they do
    /// not show up in the Signature of [`Handle`].
    /// `wait_child` is as Future that resolves when the child process/wasm process finishes
    pub fn new(
        id: usize,
        event_sender: &Sender<ProcessEvent>,
        stdin: impl AsyncWrite + Unpin + Send + 'static,
        stdout: impl AsyncRead + Unpin + Send + 'static,
        stderr: impl AsyncRead + Unpin + Send + 'static,
        wait_child: impl Future<Output=i32> + Send + 'static,
    ) -> tokio::io::Result<(UnboundedSender<ProcessCommand>, Self)> {
        let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel();

        let writer =
            tokio::task::spawn(async move { writer_task(stdin, command_receiver).await });

        let reader = {
            let sender = event_sender.clone();
            tokio::task::spawn(async move { reader_task(id, stdout, sender).await })
        };

        let log = {
            let sender = event_sender.clone();
            tokio::task::spawn(async move { log_task(id, stderr, sender).await })
        };

        let child = {
            let sender = event_sender.clone();
            tokio::task::spawn(async move { child_task(id, wait_child, sender).await })
        };

        Ok((command_sender, Self { reader, writer, log, child }))
    }

    /// Joins all tasks
    pub async fn terminate(self) {
        self.child.await
            .expect("failed to join child");
        self.reader.await
            .expect("failed to join reader thread");
        self.writer.await
            .expect("failed to join reader thread");
        self.log.await
            .expect("failed to join reader thread");
    }

    /// Returns `true` if the thread/task that waits for process exit is still running.
    pub fn is_running(&self) -> bool {
        !self.child.is_finished()
    }
}

/// Waits for [`ProcessCommand`]s and writes them to the given [`Write`]r
async fn writer_task(stdin: impl AsyncWrite + Unpin, mut receiver: UnboundedReceiver<ProcessCommand>) {
    let mut writer = BufWriter::new(stdin);
    loop {
        let Some(command) = receiver.recv().await else { break; };
        match command {
            ProcessCommand::Deliver(message) => {
                if writer.write_all(message.to_json().as_bytes()).await.is_err() {
                    todo!("react to error appropriately")
                }
                if writer.write_all(&[b'\n']).await.is_err() {
                    todo!("react to error appropriately")
                }
                if writer.flush().await.is_err() {
                    todo!("react to error appropriately")
                }
            }
        }
    }
}

/// Reads lines from the given [`Read`]er and attempts to deserialize them into [`Message`]s to send
/// them to the [`Core`](crate::core::Core).
async fn reader_task(source_id: usize, stdout: impl AsyncRead + Unpin, sender: Sender<ProcessEvent>) {
    let mut line = String::new();
    let mut reader = BufReader::new(stdout);
    let result = loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break Ok(()),
            Err(e) => break Err(e),
            Ok(_) => {}
        }
        let send_result = match Message::from_json(&line) {
            Ok(message) => sender.send(ProcessEvent::new(source_id, ProcessEventKind::Message(message))).await,
            Err(error) => sender.send(ProcessEvent::new(source_id, ProcessEventKind::SerializeError(line.clone(), error))).await
        };
        if send_result.is_err() { break Ok(()); }
    };

    warn_error(result);
}

/// Reads lines from the given [`Read`]er and sends them as log lines to the [`Core`](crate::core::Core)
async fn log_task(source_id: usize, stderr: impl AsyncRead + Unpin, sender: Sender<ProcessEvent>) {
    let mut line = String::new();
    let mut reader = BufReader::new(stderr);
    let result = loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break Ok(()),
            Err(e) => break Err(e),
            Ok(_) => {}
        }
        let line_without_newline = line.strip_suffix('\n').unwrap_or(&line);
        if sender.send(ProcessEvent::new(source_id, ProcessEventKind::Log(line_without_newline.to_owned()))).await.is_err() {
            break Ok(());
        }
    };

    warn_error(result);
}

/// Calls the given function and then sends a notification to the [`Core`](crate::core::Core)
async fn child_task(source_id: usize, wait_child: impl Future<Output=i32>, sender: Sender<ProcessEvent>) {
    let code = wait_child.await;
    sender.send(ProcessEvent::new(source_id, ProcessEventKind::Exited(code))).await.ok();
}


/// writes a warning log message if `result` is an error.
fn warn_error(result: Result<(), tokio::io::Error>) {
    if let Err(e) = result {
        if let Some(name) = std::thread::current().name() {
            log::warn!("thread \"{name}\" exited with error {e}");
        } else {
            log::warn!("thread exited with error {e}");
        }
    }
}