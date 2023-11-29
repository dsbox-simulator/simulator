//! A Handle on the threads/tasks that monitor a process.
//!
//! For each running process, four threads/tasks are spawned. Each one has a different function.
//! - one waits for [`ProcessCommand`]s from the [`Core`](crate::core::Core) and delivers them to the process
//! - one waits for lines one the processes `stdout`, attempts to deserialize them into [`Message`]s
//!   and sends them to the [`Core`](crate::core::Core).
//! - one waits for lines on the processes `stderr` and sends them to the [`Core`](crate::core::Core) as log lines.
//! - one just waits for the process to exit, and sends a notification to the [`Core`](crate::core::Core)

use std::io;
use std::io::{BufRead, BufReader, BufWriter, Error, Read, Write};
use std::path::Path;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};

use libproto::Message;

use crate::process::command::ProcessCommand;
use crate::process::event::{ProcessEvent, ProcessEventKind};

/// A handle to all four threads/tasks of a process.
pub struct Handle {
    /// handle to the thread/tasks that reads [`Message`]s from the process.d
    reader: JoinHandle<()>,
    /// handle to the thread/task that writes [`Message`]s to the process.
    writer: JoinHandle<()>,
    /// handle to the thread/task that reads log lines.
    log: JoinHandle<()>,
    /// handle to the thread/task that waits for the process to exit.
    child: JoinHandle<()>,
}

impl Handle {
    /// Creates a new [`Handle`]
    /// `stdin`, `stdout` and `stderr` are generics, because they are of a different type for native
    /// and Webassembly processes. Since they are only passed off to different threads, they do
    /// not show up in the Signature of [`Handle`].
    /// `wait_child` is a function that should block until the process exits. For native processes
    /// it just waits for the child process to exit, for Webassembly this is a callback that
    /// actually calls the `start` function in the loaded Webassembly.
    pub fn new(
        id: usize,
        path: &Path,
        event_sender: &Sender<ProcessEvent>,
        stdin: impl Write + Send + 'static,
        stdout: impl Read + Send + 'static,
        stderr: impl Read + Send + 'static,
        wait_child: impl FnOnce() -> i32 + Send + 'static,
    ) -> io::Result<(Sender<ProcessCommand>, Self)> {
        let (command_sender, command_receiver) = crossbeam_channel::unbounded();

        let writer = {
            std::thread::Builder::new()
                .name(format!("[w-{id}] {}", path.display()))
                .spawn(move || { writer_thread(stdin, command_receiver) })?
        };

        let reader = {
            let sender = event_sender.clone();
            std::thread::Builder::new()
                .name(format!("[r-{id}] {}", path.display()))
                .spawn(move || { reader_thread(id, stdout, sender) })?
        };

        let log = {
            let sender = event_sender.clone();
            std::thread::Builder::new()
                .name(format!("[l-{id}] {}", path.display()))
                .spawn(move || { log_thread(id, stderr, sender) })?
        };

        let child = {
            let sender = event_sender.clone();
            std::thread::Builder::new()
                .name(format!("[c-{id}] {}", path.display()))
                .spawn(move || { child_thread(id, wait_child, sender) })?
        };

        Ok((command_sender, Self { reader, writer, log, child }))
    }

    /// Joins all threads/tasks
    pub fn terminate(self) {
        self.child.join()
            .expect("failed to join child");
        self.reader.join()
            .expect("failed to join reader thread");
        self.writer.join()
            .expect("failed to join reader thread");
        self.log.join()
            .expect("failed to join reader thread");
    }

    /// Returns `true` if the thread/task that waits for process exit is still running.
    pub fn is_running(&self) -> bool {
        !self.child.is_finished()
    }
}

/// Waits for [`ProcessCommand`]s and writes them to the given [`Write`]r
fn writer_thread(stdin: impl Write, receiver: Receiver<ProcessCommand>) {
    let mut writer = BufWriter::new(stdin);
    loop {
        let Ok(command) = receiver.recv() else { break; };
        match command {
            ProcessCommand::Deliver(message) => {
                if writer.write_all(message.to_json().as_bytes()).is_err() {
                    todo!("react to error appropriately")
                }
                if writer.write_all(&[b'\n']).is_err() {
                    todo!("react to error appropriately")
                }
                if writer.flush().is_err() {
                    todo!("react to error appropriately")
                }
            }
        }
    }
}

/// Reads lines from the given [`Read`]er and attempts to deserialize them into [`Message`]s to send
/// them to the [`Core`](crate::core::Core).
fn reader_thread(source_id: usize, stdout: impl Read, sender: Sender<ProcessEvent>) {
    let mut line = String::new();
    let mut reader = BufReader::new(stdout);
    let result = loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break Ok(()),
            Err(e) => break Err(e),
            Ok(_) => {}
        }
        let send_result = match Message::from_json(&line) {
            Ok(message) => sender.send(ProcessEvent::new(source_id, ProcessEventKind::Message(message))),
            Err(error) => sender.send(ProcessEvent::new(source_id, ProcessEventKind::SerializeError(line.clone(), error)))
        };
        if send_result.is_err() { break Ok(()); }
    };

    warn_error(result);
}

/// Reads lines from the given [`Read`]er and sends them as log lines to the [`Core`](crate::core::Core)
fn log_thread(source_id: usize, stderr: impl Read, sender: Sender<ProcessEvent>) {
    let mut line = String::new();
    let mut reader = BufReader::new(stderr);
    let result = loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break Ok(()),
            Err(e) => break Err(e),
            Ok(_) => {}
        }
        let line_without_newline = line.strip_suffix('\n').unwrap_or(&line);
        if sender.send(ProcessEvent::new(source_id, ProcessEventKind::Log(line_without_newline.to_owned()))).is_err() {
            break Ok(());
        }
    };

    warn_error(result);
}

/// Calls the given function and then sends a notification to the [`Core`](crate::core::Core)
fn child_thread(source_id: usize, wait_child: impl FnOnce() -> i32, sender: Sender<ProcessEvent>) {
    let code = wait_child();
    sender.send(ProcessEvent::new(source_id, ProcessEventKind::Exited(code))).ok();
}


/// writes a warning log message if `result` is an error.
fn warn_error(result: Result<(), Error>) {
    if let Err(e) = result {
        if let Some(name) = std::thread::current().name() {
            log::warn!("thread \"{name}\" exited with error {e}");
        } else {
            log::warn!("thread exited with error {e}");
        }
    }
}