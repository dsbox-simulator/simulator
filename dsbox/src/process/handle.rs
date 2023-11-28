use std::io;
use std::io::{BufRead, BufReader, BufWriter, Error, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};

use libproto::Message;
use crate::process::command::ProcessCommand;
use crate::process::event::{ProcessEvent, ProcessEventKind};

pub struct Handle {
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
    log: JoinHandle<()>,
    child: JoinHandle<()>,
}

static SPAWN_ID: AtomicUsize = AtomicUsize::new(0);

impl Handle {
    pub fn new(
        id: usize,
        file: &Path,
        event_sender: &Sender<ProcessEvent>,
        stdin: impl Write + Send + 'static,
        stdout: impl Read + Send + 'static,
        stderr: impl Read + Send + 'static,
        wait_child: impl FnOnce() -> i32 + Send + 'static,
    ) -> io::Result<(Sender<ProcessCommand>, Self)> {
        let (command_sender, command_receiver) = crossbeam_channel::unbounded();

        let spawn_id = SPAWN_ID.fetch_add(1, Ordering::SeqCst);

        let writer = {
            std::thread::Builder::new()
                .name(format!("[w-{spawn_id}] {}", file.display()))
                .spawn(move || { writer_thread(stdin, command_receiver) })?
        };

        let reader = {
            let sender = event_sender.clone();
            std::thread::Builder::new()
                .name(format!("[r-{spawn_id}] {}", file.display()))
                .spawn(move || { reader_thread(id, stdout, sender) })?
        };

        let log = {
            let sender = event_sender.clone();
            std::thread::Builder::new()
                .name(format!("[l-{spawn_id}] {}", file.display()))
                .spawn(move || { log_thread(id, stderr, sender) })?
        };

        let child = {
            let sender = event_sender.clone();
            std::thread::Builder::new()
                .name(format!("[c-{spawn_id}] {}", file.display()))
                .spawn(move || { child_thread(id, wait_child, sender) })?
        };

        Ok((command_sender, Self { reader, writer, log, child }))
    }

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

    pub fn is_running(&self) -> bool {
        !self.child.is_finished()
    }
}

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

fn child_thread(source_id: usize, wait_child: impl FnOnce() -> i32, sender: Sender<ProcessEvent>) {
    let code = wait_child();
    sender.send(ProcessEvent::new(source_id, ProcessEventKind::Exited(code))).ok();
}


fn warn_error(result: Result<(), Error>) {
    if let Err(e) = result {
        if let Some(name) = std::thread::current().name() {
            log::warn!("thread \"{name}\" exited with error {e}");
        } else {
            log::warn!("thread exited with error {e}");
        }
    }
}