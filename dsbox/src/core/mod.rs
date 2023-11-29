//! The core of the simulation, that governs the execution of the simulation
//!
//! The [`Core`] contains all logic to handle communication between nodes, handling of core [`Message`]s,
//! handling of [`RemoteCommand`]s and publishing of all [`Event`]s.
//!
//! As such it is also the point at which all [`ProcessEvent`]s are serialized into a definite order.
//! This order is itself non-deterministic and can change across multiple executions.
//!
//! When a [`Core`] is created, a single client process is launched. During execution the client process
//! can then send a [`Setup`] message to the core, to give itself one or more (client-) node names, and launch
//! a number of server nodes, with given names. Each server node is then sent an [`Init`] message with
//! its own name and a list of the other server names (which includes itself).
//! After successfully launching and initializing all server nodes, the client process is sent a [`SetupOk`] message.
//!
//! When the client process sends a new [`Setup`] message, the core closes all communications with
//! existing server processes and waits for them to exit.
//! It also clears all existing server and client names, as well as all running [`MonitorSession`]s.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crossbeam_channel::{Receiver, Sender};

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
use crate::process::{Process, ProcessCommand, ProcessEvent, ProcessEventKind};
use crate::protocol::{Protocol, ProtocolSubscriber};
use crate::timestamp::Timestamp;

mod process_manager;
pub mod error;
pub mod remote_control;
mod monitor;
pub mod event;

/// The core of the simulation
///
/// This struct contains all state of the simulation and is used to drive execution forwards
/// by collecting [`ProcessEvent`]s from processes, delivering [`Message`]s and listening for
/// remote control commands.
pub struct Core {
    /// Manages all processes that are participating in the simulation
    processes: ProcessManager,
    /// Receives [`ProcessEvent`]s. The corresponding [`Sender`] is passed to the [`ProcessManager`] for processes to send their [`ProcessEvent`]s to the core.
    receiver: Receiver<ProcessEvent>,
    /// Path to the executable file from which server processes are launched.
    server_path: PathBuf,
    /// The current execution state (i.e. running/stepping/paused...)
    state: CoreState,
    /// Receives [`RemoteCommand`]s for controlling this [`Core`]
    remote_receiver: Receiver<RemoteCommand>,
    /// is cloned and given to whoever wants to remote control this [`Core`].
    remote_sender: Sender<RemoteCommand>,
    /// used for recording all [`Event`]s and publishing them to potential subscribers (like the web app)
    protocol: Protocol,
    /// list of all active [`MonitorSession`]s
    monitor_sessions: Vec<MonitorSession>,
    /// the [`Network`] contains all [`Message`]s that are sent, but not yet delivered
    network: Network,
}

/// The execution state for a [`Core`]
enum CoreState {
    /// The [`Core`] is running normally.
    Running,
    /// The delivery of [`Message`]s is paused. Everything else (including the sending of [`Message`]s) continues normally.
    Paused,
    /// The [`Core`] is stepping, i.e. it will wait for and deliver a single [`Message`] to a node, and then return to [`CoreState::Paused`].
    Stepping,
}

/// The "node name" of the [`Core`]. It is used by clients to send core messages (i.e. [`Setup`])
const CORE_NAME: &'static str = "core";

/// The "node name" of the client process. It is used by the [`Core`] to send messages to the client process that are not specific to a client node (i.e. [`SetupOk`])
const CLIENT_NAME: &'static str = "client";

impl Core {
    /// Creates a new [`Core`] from the given cli arguments (which include the server and client executables among other things).
    /// If the program is started in interactive mode, the [`Core`] starts in state [`CoreState::Paused`].
    pub fn new(args: &Args) -> Result<Self, CoreError> {
        let (sender, receiver) = crossbeam_channel::bounded(0);
        let mut processes = ProcessManager::new(sender);
        processes.launch(Path::new(&args.test_path))
            .map_err(|e| CoreError::LaunchFailed(PathBuf::from(&args.test_path), e))?;

        let (remote_sender, remote_receiver) = crossbeam_channel::bounded(0);

        Ok(Self {
            processes,
            receiver,
            server_path: PathBuf::from(&args.server_path),
            state: if args.interactive { CoreState::Paused } else { CoreState::Running },
            remote_sender,
            remote_receiver,
            protocol: Protocol::new(),
            monitor_sessions: Vec::new(),
            network: Network::new(),
        })
    }

    /// Returns a new [`Sender`] for sending [`RemoteCommand`]s to the [`Core`]
    pub fn remote_control(&self) -> Sender<RemoteCommand> {
        self.remote_sender.clone()
    }

    /// Returns a new [`ProtocolSubscriber`] for listening to events from the [`Core`]
    pub fn subscribe_events(&self) -> ProtocolSubscriber {
        self.protocol.subscribe()
    }

    /// starts the execution. This function consumes the passed [`Core`] because it cannot be restarted
    /// after [`Core::run`] returns.
    pub fn run(mut self) -> Result<(), CoreError> {
        loop {
            if !self.processes.iter().any(Process::is_running) { break; }

            self.step()?;
        }

        Ok(())
    }

    fn get_next_message_for_delivery(&mut self) -> Option<(Timestamp, Message)> {
        if !matches!(self.state, CoreState::Paused) {
            self.network.remove_oldest()
        } else {
            None
        }
    }

    fn step(&mut self) -> Result<(), CoreError> {
        if let Some((sent_timestamp, message)) = self.get_next_message_for_delivery() {
            self.deliver(sent_timestamp, message)?;
            if matches!(self.state, CoreState::Stepping) {
                self.state = CoreState::Paused;
            }
        } else {
            crossbeam_channel::select! {
                recv(self.remote_receiver) -> remote_command => {
                    self.handle_command(remote_command.unwrap());
                }
                recv(self.receiver) -> process_event => {
                    self.handle_process_event(Timestamp::now(), process_event.unwrap())?;
                }
            }
        }
        Ok(())
    }

    /// handles a single [`RemoteCommand`]
    fn handle_command(&mut self, command: RemoteCommand) {
        match command {
            RemoteCommand::Pause => self.state = CoreState::Paused,
            RemoteCommand::Step => self.state = CoreState::Stepping,
            RemoteCommand::Resume => self.state = CoreState::Running,
        }
    }

    /// Handles a single [`ProcessEvent`].
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
                if process_event.source_id == 0 {
                    // process 0 exited: shut down all processes gracefully
                    for proc in &mut self.processes {
                        proc.begin_shutdown();
                    }
                }
                Ok(true)
            }
            ProcessEventKind::SerializeError(raw_message, err) => {
                Err(CoreError::SerializeError(self.processes[process_event.source_id].path().to_path_buf(), raw_message, err))
            }
        }
    }

    /// Dispatches a single [`Message`] into the network.
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
        self.protocol.publish_event(Event::send_message(timestamp, message.clone()));
        self.network.insert(timestamp, message);
        Ok(())
    }

    /// Delivers a single [`Message`] to the destination node.
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
        self.protocol.publish_event(Event::deliver_message(timestamp, sent_timestamp.logical));
        self.processes[destination_id].send(ProcessCommand::Deliver(message));
        Ok(())
    }

    /// Checks whether any active monitoring session matches against the given [`Message`], and sends a [`MonitorEvent`]
    /// to the corresponding source node. If `in_reference_to` is `None`, the kind is [`MonitorEventKind::Sent`], otherwise
    /// it is [`MonitorEventKind::Delivered`].
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

    /// handles a single core [`Message`] (i.e. [`Setup`] or [`BeginMonitor`]).
    /// Returns an error if the [`Message`] was not send from a client node, if the [`Message`]s type
    /// is not known, or if handling of the [`Message`] itself fails.
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
            ty => Err(CoreError::UnknownCoreMessage(ty.to_owned()))
        }
    }

    /// Resets the [`Core`] and sets up a new test run with the given nodes in the [`Setup`][`Message`].
    fn setup(&mut self, setup: Setup) -> Result<(), CoreError> {
        self.processes.reset_names();
        self.monitor_sessions.clear();

        let mut nodes = HashMap::new();

        // all existing server nodes can shut down now
        // technically it would not be a problem to just leave them be, but this just
        // seems a little cleaner. Once the client process exits, all processes are shut down anyways.
        for proc in &mut self.processes[1..] {
            proc.begin_shutdown();
        }

        for client_name in setup.clients {
            self.processes.add_name(client_name.clone(), 0);
            nodes.insert(client_name, 0);
        };

        for name in &setup.servers {
            let id = self.processes.launch(&self.server_path)
                .map_err(|e| CoreError::LaunchFailed(self.server_path.clone(), e))?;
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
        self.protocol.publish_event(Event::setup(Timestamp::now(), nodes));
        Ok(())
    }

    /// Sends a log event to all subscribers and writes the line to the current logger implementation.
    fn log(&mut self, timestamp: Timestamp, source_id: usize, line: String) -> Result<(), CoreError> {
        let source_path = self.processes[source_id].path();
        log::info!("[{}]: {line}", source_path.display());
        self.protocol.publish_event(Event::log(timestamp, source_id, source_path.to_owned(), line));
        Ok(())
    }

    /// Sends a disconnect event to all subscribers and logs the exit code of the process.
    fn process_exited(&mut self, timestamp: Timestamp, source_id: usize, exit_code: i32) -> Result<(), CoreError> {
        self.protocol.publish_event(Event::node_disconnected(timestamp, source_id));
        log::info!("process {} exited with code {exit_code}", self.processes[source_id].path().display());
        Ok(())
    }
}
