use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use crossbeam_channel::{Receiver, Sender};
use tokio::sync::broadcast;

use libproto::{Message, Payload};
use libproto::init::Init;
use libproto::system::{BeginMonitor, MonitorEvent, MonitorEventKind, Setup, SetupOk};

use crate::cli::Args;
use crate::core::error::{CoreError, DispatchErrorKind};
use crate::core::event::Event;
use crate::core::monitor::MonitorSession;
use crate::core::process_manager::ProcessManager;
use crate::core::remote_control::RemoteCommand;
use crate::network::Network;
use crate::process::{ProcessCommand, ProcessEvent, ProcessEventKind};
use crate::timestamp::Timestamp;

mod process_manager;
pub mod error;
pub mod remote_control;
mod monitor;
pub mod event;

pub struct Core {
    processes: ProcessManager,
    receiver: Receiver<ProcessEvent>,
    server_path: PathBuf,
    state: CoreState,
    remote_receiver: Receiver<RemoteCommand>,
    remote_sender: Sender<RemoteCommand>,
    event_sender: broadcast::Sender<Event>,
    process_event_queue: VecDeque<(Timestamp, ProcessEvent)>,
    monitor_sessions: Vec<MonitorSession>,
    network: Network,
}

enum CoreState {
    Running,
    Paused,
    Stepping,
}

const CORE_NAME: &'static str = "core";
const CLIENT_NAME: &'static str = "client";

impl Core {
    pub fn new(args: &Args) -> Result<Self, CoreError> {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut processes = ProcessManager::new(sender);
        processes.spawn(Path::new(&args.test_path))
            .map_err(|e| CoreError::SpawnFailed(PathBuf::from(&args.test_path), e))?;

        let (remote_sender, remote_receiver) = crossbeam_channel::unbounded();
        let (event_sender, _) = broadcast::channel(16);

        Ok(Self {
            processes,
            receiver,
            server_path: PathBuf::from(&args.server_path),
            state: if args.interactive { CoreState::Paused } else { CoreState::Running },
            remote_sender,
            remote_receiver,
            event_sender,
            process_event_queue: VecDeque::new(),
            monitor_sessions: Vec::new(),
            network: Network::new(),
        })
    }

    pub fn remote_control(&self) -> Sender<RemoteCommand> {
        self.remote_sender.clone()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<Event> {
        self.event_sender.subscribe()
    }

    pub fn run(mut self) -> Result<(), CoreError> {
        loop {
            if !self.processes[0].is_running() { break; }

            if let Some(command) = self.pump_events() {
                self.handle_command(command)
            } else {
                self.run_state()?;
            }
        }

        for mut process in self.processes {
            process.terminate();
        }

        Ok(())
    }

    fn handle_command(&mut self, command: RemoteCommand) {
        match command {
            RemoteCommand::Pause => self.state = CoreState::Paused,
            RemoteCommand::Step => self.state = CoreState::Stepping,
            RemoteCommand::Resume => self.state = CoreState::Running,
        }
    }

    fn pump_events(&mut self) -> Option<RemoteCommand> {
        if self.is_idle() {
            // nothing else to do for now: just wait for an event or a command to come in
            crossbeam_channel::select! {
                recv(self.remote_receiver) -> remote_command => {
                    Some(remote_command.unwrap())
                }
                recv(self.receiver) -> process_event => {
                    self.process_event_queue.push_back((Timestamp::now(), process_event.unwrap()));
                    while let Ok(event) = self.receiver.try_recv() {
                        self.process_event_queue.push_back((Timestamp::now(), event));
                    }
                    None
                }
            }
        } else {
            // other things are still left to do: check for a remote command and pump new events if any
            if let Ok(command) = self.remote_receiver.try_recv() {
                return Some(command);
            }

            while let Ok(event) = self.receiver.try_recv() {
                self.process_event_queue.push_back((Timestamp::now(), event));
            }
            None
        }
    }

    fn is_idle(&self) -> bool {
        self.process_event_queue.is_empty() && self.network.is_empty()
    }

    fn run_state(&mut self) -> Result<(), CoreError> {
        match self.state {
            CoreState::Running => self.step(),
            CoreState::Paused => self.step_paused(),
            CoreState::Stepping => {
                self.step()?;
                self.state = CoreState::Paused;
                Ok(())
            }
        }
    }

    fn step(&mut self) -> Result<(), CoreError> {
        loop {
            if let Some((timestamp, process_event)) = self.process_event_queue.pop_front() {
                let handle_more = self.handle_process_event(timestamp, process_event)?;
                if handle_more { continue; }
            } else if let Some((timestamp, message)) = self.network.remove_oldest() {
                self.deliver(timestamp, message)?;
            }
            break;
        }
        Ok(())
    }

    fn step_paused(&mut self) -> Result<(), CoreError> {
        // keep message events in the queue, but process any other event (like log messages etc.)
        let capacity = self.process_event_queue.len();
        let event_queue = std::mem::replace(&mut self.process_event_queue, VecDeque::with_capacity(capacity));
        for (timestamp, process_event) in event_queue {
            if !matches!(process_event.kind, ProcessEventKind::Message(_)) {
                self.handle_process_event(timestamp, process_event)?;
            } else {
                self.process_event_queue.push_back((timestamp, process_event));
            }
        }
        Ok(())
    }

    fn handle_process_event(&mut self, timestamp: Timestamp, process_event: ProcessEvent) -> Result<bool, CoreError> {
        match process_event.kind {
            ProcessEventKind::Message(message) => {
                self.dispatch(process_event.source_id, timestamp, message)?;
                Ok(false)
            }
            ProcessEventKind::Log(log) => {
                self.log(timestamp, process_event.source_id, log)?;
                Ok(true)
            }
            ProcessEventKind::Exited(exit_code) => {
                self.process_exited(timestamp, process_event.source_id, exit_code)?;
                Ok(true)
            }
            ProcessEventKind::SerializeError(raw_message, err) => {
                Err(CoreError::SerializeError(self.processes[process_event.source_id].path().to_path_buf(), raw_message, err))
            }
        }
    }

    fn dispatch(&mut self, source_id: usize, timestamp: Timestamp, message: Message) -> Result<(), CoreError> {
        log::trace!("dispatching message: {}", message.to_json());
        if message.dst == CORE_NAME {
            return self.handle_core_message(source_id, message);
        }

        let source = &self.processes[source_id];
        if !self.processes.has_name(source, &message.src) {
            return Err(CoreError::DispatchError {
                source: source.path().to_path_buf(),
                message,
                kind: DispatchErrorKind::SourceNameMismatch,
            });
        }

        self.send_monitor_event(timestamp, &message, None);
        self.event_sender.send(Event::send_message(timestamp, message.clone())).ok();
        self.network.insert(timestamp, message);
        Ok(())
    }

    fn deliver(&mut self, sent_timestamp: Timestamp, message: Message) -> Result<(), CoreError> {
        let Some(destination_id) = self.processes.id_by_name(&message.dst) else {
            let source_id = self.processes.id_by_name(&message.src).unwrap();
            return Err(CoreError::DispatchError {
                source: self.processes[source_id]
                    .path()
                    .to_path_buf(),
                message,
                kind: DispatchErrorKind::DestinationUnknown,
            });
        };
        let timestamp = Timestamp::now();
        self.send_monitor_event(timestamp, &message, Some(sent_timestamp.logical));
        self.event_sender.send(Event::deliver_message(timestamp, sent_timestamp.logical)).ok();
        self.processes[destination_id].send(ProcessCommand::Deliver(message));
        Ok(())
    }

    fn send_monitor_event(&mut self, timestamp: Timestamp, message: &Message, in_reference_to: Option<usize>) {
        for session in &self.monitor_sessions {
            if session.matches(&message) {
                let source_id = self.processes.id_by_name(session.source()).unwrap();
                self.processes[source_id]
                    .send(ProcessCommand::Deliver(Message::new(CORE_NAME, session.source(), None, MonitorEvent {
                        kind: if in_reference_to.is_some() { MonitorEventKind::Delivered } else { MonitorEventKind::Sent },
                        timestamp_logical: timestamp.logical,
                        timestamp_physical: timestamp.physical,
                        in_reference_to,
                        message: message.clone(),
                    })));
            }
        }
    }

    fn handle_core_message(&mut self, source_id: usize, message: Message) -> Result<(), CoreError> {
        if source_id != 0 {
            return Err(CoreError::IllegalCoreMessage(self.processes[source_id].path().to_path_buf(), message));
        }
        match message.body.ty.as_str() {
            Setup::TYPE => {
                self.setup(message.payload::<Setup>().unwrap())?;
                Ok(())
            }
            BeginMonitor::TYPE => {
                let begin_monitor = message.payload::<BeginMonitor>().unwrap();
                let session = match MonitorSession::new(message.src, &begin_monitor.src_match, &begin_monitor.dst_match) {
                    Ok(session) => session,
                    Err(err) => {
                        log::warn!("failed to start monitor session, source or destination expression invalid: {err}");
                        return Ok(());
                    }
                };
                self.monitor_sessions.push(session);
                Ok(())
            }
            ty => Err(CoreError::UnknownSystemMessage(ty.to_owned()))
        }
    }

    fn setup(&mut self, setup: Setup) -> Result<(), CoreError> {
        self.processes.reset_names();
        self.monitor_sessions.clear();

        let mut nodes = HashMap::new();

        for proc in &mut self.processes[1..] {
            if proc.is_running() {
                log::info!("waiting for node {} (process {}) to exit", proc.id(), proc.path().display());
                proc.terminate();
            }
        }

        for client_name in setup.clients {
            self.processes.add_name(client_name.clone(), 0);
            nodes.insert(client_name, 0);
        };

        for name in &setup.servers {
            let id = self.processes.spawn(&self.server_path)
                .map_err(|e| CoreError::SpawnFailed(self.server_path.clone(), e))?;
            self.processes.add_name(name.clone(), id);
            nodes.insert(name.clone(), id);
        }

        for name in &setup.servers {
            let server_id = self.processes.id_by_name(name).unwrap();
            self.processes[server_id].send(ProcessCommand::Deliver(Message::new(CORE_NAME, name, None, Init {
                name: name.clone(),
                servers: setup.servers.clone(),
            })));
        }
        self.processes[0].send(ProcessCommand::Deliver(Message::new(CORE_NAME, CLIENT_NAME, None, SetupOk {})));
        self.event_sender.send(Event::setup(Timestamp::now(), nodes)).ok();
        Ok(())
    }

    fn log(&mut self, timestamp: Timestamp, source_id: usize, line: String) -> Result<(), CoreError> {
        let source_path = self.processes[source_id].path();
        log::info!("[{}]: {line}", source_path.display());
        self.event_sender.send(Event::log(timestamp, source_id, source_path.to_owned(), line)).ok();
        Ok(())
    }

    fn process_exited(&mut self, timestamp: Timestamp, source_id: usize, exit_code: i32) -> Result<(), CoreError> {
        self.event_sender.send(Event::node_disconnected(timestamp, source_id)).ok();
        log::info!("process {} exited with code {exit_code}", self.processes[source_id].path().display());
        Ok(())
    }
}
