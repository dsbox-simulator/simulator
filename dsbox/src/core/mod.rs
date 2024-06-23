//! The core of the simulation, that governs the execution of the simulation
//!
//! The [`Core`] contains all logic to handle communication between nodes, handling of core [`Message`]s,
//! handling of [`RemoteCommand`]s and publishing of all [`Event`]s.
//!
//! As such it is also the point at which all [`ProcessEvent`]s are serialized into a definite order.
//! This order is itself non-deterministic and can change across multiple executions.
//!
//! When a [`Core`] is created, a single client process is launched. During execution the client process
//! can then send a number of [`Launch`] messages to the core, to give itself one or more (client-) node names, or launch
//! a number of server nodes, with given names. Each server node is then sent an [`Init`] message with its own name.
//!
//! After launching and initializing a node, the client process is sent a [`LaunchFinished`] message.
//!
//! When the client process sends a [`Reset`] message, the core closes all communications with
//! existing server processes and waits for them to exit.
//! It also clears all existing server and client names, as well as all running [`MonitorSession`]s.

use std::collections::VecDeque;
use std::slice::SliceIndex;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::{Duration, Instant};

use libproto::{Message, Payload};
use libproto::init::Init;
use libproto::middleware::{Forward, Next};
use libproto::services::{LogMessage, TimerExpired};
use libproto::system::{BeginMonitor, Break, Launch, LaunchFinished, MonitorEvent, MonitorEventKind, Reset, ResetFinished};
use node::Node;

use crate::cli::Args;
use crate::core::error::{CoreError, DispatchErrorKind};
use crate::core::event::Event;
use crate::core::monitor::MonitorSession;
use crate::core::node::{MiddlewareId, NodeId};
use crate::core::node_list::NodeList;
use crate::core::remote_control::RemoteCommand;
use crate::core::timer_manager::{Timer, TimerManager};
use crate::log_color;
use crate::log_color::log_marker_ansi_color;
use crate::network::Network;
use crate::process::{Launcher, Process, ProcessCommand, ProcessEvent};
use crate::protocol::{Protocol, ProtocolSubscriber};
use crate::timestamp::Timestamp;

mod monitor;
mod node_list;
mod node;
pub mod error;
pub mod event;
pub mod remote_control;
mod timer_manager;

/// The core of the simulation
///
/// This struct contains all state of the simulation and is used to drive execution forwards
/// by collecting [`ProcessEvent`]s from processes, delivering [`Message`]s and listening for
/// remote control commands.
pub struct Core {
    /// Manages all nodes that are participating in the simulation
    nodes: NodeList,
    /// launches new processes
    launcher: Launcher,
    /// Command string from which server processes are launched.
    server_command: String,
    /// `true` if the program was started in interactive mode (i.e. with the user interface enabled)
    interactive: bool,
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
    /// a manager for outstanding timers
    timer_manager: TimerManager,
    /// queue of [`Launch`] messages to be launched at some later point
    /// (used to prevent recursion in the async [`dispatch`](Core::dispatch) fn because launching a node
    /// requires dispatching an init message to that node)
    launch_queue: VecDeque<Launch>,
    /// when the client sends a reset message, some stuff has to be finished/cleaned up before
    /// the new nodes may be launched,
    /// so we set this flag while we wait for everything to become ready
    reset: bool,
}

/// The execution state for a [`Core`]
#[derive(Copy, Clone, Eq, PartialEq)]
enum CoreState {
    /// The [`Core`] is running normally.
    Running,
    /// The delivery of [`Message`]s is paused. Everything else (including the sending of [`Message`]s) continues normally.
    Paused,
    /// The [`Core`] is stepping, i.e. it will wait for and deliver a single [`Message`] to a node, and then return to [`CoreState::Paused`].
    Stepping,
}

/// The "node name" of the [`Core`]. It is used by clients to send core messages (i.e. [`Launch`])
pub const CORE_NAME: &'static str = "core";

/// The "node name" of the client process. It is used by the [`Core`] to send messages to the client process that are not specific to a client node (i.e. [`LaunchFinished`])
const CLIENT_NAME: &'static str = "client";

impl Core {
    /// Creates a new [`Core`] from the given cli arguments (which include the server and client executables among other things).
    /// If the program is started in interactive mode, the [`Core`] starts in state [`CoreState::Paused`].
    pub async fn new(args: &Args) -> Result<Self, CoreError> {
        let (remote_sender, remote_receiver) = tokio::sync::mpsc::channel(1);

        let mut core = Self {
            nodes: NodeList::new(),
            launcher: Launcher::new(args),
            server_command: args.server_command.clone(),
            interactive: args.interactive,
            state: if args.interactive { CoreState::Paused } else { CoreState::Running },
            remote_sender,
            remote_receiver,
            protocol: Protocol::new(),
            monitor_sessions: Vec::new(),
            network: Network::new(),
            timer_manager: TimerManager::new(),
            launch_queue: VecDeque::new(),
            reset: false,
        };
        assert_eq!(core.launch(Some(&args.test_command), true, CLIENT_NAME.to_string()).await?.id, NodeId(0), "expected client to have id 0");
        Ok(core)
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
    pub async fn run(mut self) -> Result<(), CoreError> {
        let mut deadline = None;
        loop {
            let mut num_running = 0;
            let mut num_servers = 0;
            for node in &self.nodes {
                if node.has_finished() { continue; }
                num_running += 1;
                if !node.is_client { num_servers += 1; }
            }
            if num_running == 0 { break; }

            let dont_block = deadline.is_some() || !self.launch_queue.is_empty();

            self.step(dont_block).await?;
            if let Some(launch) = self.launch_queue.pop_front() {
                self.launch_single(launch).await?;
            } else if self.reset && self.network.is_empty() {
                if deadline.is_none() {
                    deadline = Some(Instant::now() + Duration::from_secs(1));
                } else if num_servers == 0 || Instant::now() > deadline.unwrap() {
                    deadline = None;
                    self.reset = false;
                    self.reset().await?
                }
            }
        }
        let deadline = Instant::now() + Duration::from_secs(1);
        for node in self.nodes.drain(..) {
            tokio::time::timeout_at(deadline, node.terminate()).await.ok();
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

    async fn step(&mut self, dont_block: bool) -> Result<(), CoreError> {
        // TODO: if processes spam messages and never receive any, they can force the receiving
        //       queues to fill up, which will waste a lot of RAM and possibly de-stabilize the system
        //       possible solution: before picking up a message from a process, check if the other
        //       ends' receiving queue has space for that message? (could probably lead to deadlocks in tricky situations)
        //       other possible solution: regularly check receiving queues of all processes, and if they
        //       reach a total threshold of buffered messages (say, 1,000,000) panic as a last resort?

        if let Some((sent_timestamp, message)) = self.get_next_message_for_delivery() {
            self.deliver(sent_timestamp, message).await?;
            if matches!(self.state, CoreState::Stepping) {
                self.state = CoreState::Paused;
            }
        } else {
            let timeout = async move { if dont_block { tokio::time::sleep(Duration::from_millis(10)).await } };
            tokio::select! {
                biased;
                remote_command = self.remote_receiver.recv() => {
                    self.handle_command(remote_command.unwrap()).await?;
                }
                process_event = self.nodes.recv_any() => {
                    if let Some((event, node_id, middleware_idx)) = process_event {
                        self.handle_process_event(Timestamp::now(), node_id, middleware_idx, event).await?;
                    }
                }
                timer = self.timer_manager.wait_next() => {
                    self.send_timer_expired(timer).await?;
                }
                _ = timeout, if dont_block => {}
            }
        }
        Ok(())
    }

    /// Handles a single [`ProcessEvent`].
    async fn handle_process_event(&mut self, timestamp: Timestamp, node_id: NodeId, middleware_id: MiddlewareId, process_event: ProcessEvent) -> Result<bool, CoreError> {
        match process_event {
            ProcessEvent::Message(message) => {
                self.dispatch(Some((node_id, middleware_id)), timestamp, message).await?;
                Ok(false)
            }
            ProcessEvent::Log(log) => {
                let log_message = LogMessage {
                    text: log,
                    marker: None,
                };
                self.log(timestamp, node_id, middleware_id, log_message).await?;
                Ok(true)
            }
            ProcessEvent::Exited(exit_code) => {
                self.process_exited(timestamp, node_id, middleware_id, exit_code).await?;
                if self.nodes[node_id].is_client {
                    // client process exited: shut down all processes gracefully
                    self.begin_shutdown(..)?;
                }
                Ok(true)
            }
            ProcessEvent::SerializeError { raw_message, error } => {
                Err(CoreError::SerializeError {
                    source: self.nodes[node_id].commandline(middleware_id),
                    raw_message,
                    error,
                })
            }
        }
    }

    /// Dispatches a single [`Message`] into the network.
    async fn dispatch(&mut self, source: Option<(NodeId, MiddlewareId)>, timestamp: Timestamp, message: Message) -> Result<(), CoreError> {
        log::trace!("dispatching message {}", message.to_json());

        if let Some((source_id, _)) = source {
            let source = &self.nodes[source_id];
            if !self.nodes.is_alias_of(source, &message.src) {
                return Err(CoreError::DispatchError {
                    name: source.name.clone(),
                    message,
                    kind: DispatchErrorKind::SourceNameMismatch,
                });
            }
        }
        self.protocol.publish_event(Event::send_message(timestamp, message.clone())).await;

        if message.dst == CORE_NAME {
            return self.handle_core_message(source, timestamp, message).await;
        }

        if let Some((source_id, middleware_id)) = source {
            let node = &self.nodes[source_id];
            if !middleware_id.is_top() {
                node.send_to_middleware(ProcessCommand::Deliver(Message::new("core", &node.name, None, Forward { message })), middleware_id.above());
                return Ok(());
            }
        }

        self.send_monitor_event(timestamp, &message, None).await;
        self.network.insert(timestamp, message);
        Ok(())
    }

    /// Delivers a single [`Message`] to the destination node.
    async fn deliver(&mut self, sent_timestamp: Timestamp, message: Message) -> Result<(), CoreError> {
        let Some(destination_id) = self.nodes.lookup(&message.dst).map(|n| n.id) else {
            let source_node = self.nodes.lookup(&message.src).unwrap();
            return Err(CoreError::DispatchError {
                name: source_node.name.clone(),
                message,
                kind: DispatchErrorKind::DestinationUnknown,
            });
        };
        let timestamp = Timestamp::now();
        self.send_monitor_event(timestamp, &message, Some(sent_timestamp.logical)).await;
        self.protocol.publish_event(Event::deliver_message(timestamp, sent_timestamp.logical)).await;
        self.nodes[destination_id].send(ProcessCommand::Deliver(message));
        Ok(())
    }

    /// Checks whether any active monitoring session matches against the given [`Message`], and sends a [`MonitorEvent`]
    /// to the corresponding source node. If `in_reference_to` is `None`, the kind is [`MonitorEventKind::Sent`], otherwise
    /// it is [`MonitorEventKind::Delivered`].
    async fn send_monitor_event(&mut self, timestamp: Timestamp, message: &Message, in_reference_to: Option<usize>) {
        for session in &self.monitor_sessions {
            if session.matches(&message) {
                let monitor_node = self.nodes.lookup(session.source()).unwrap();
                // monitor events are not dispatched via the network. Instead, they are delivered directly
                // to the target node. Among other reasons, this de-clutters the message log (monitor events
                // should not be the target of any kind of debugging/visualization)
                monitor_node
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

    async fn send_timer_expired(&mut self, timer: Timer) -> Result<(), CoreError> {
        let mut reply = Message::new(CORE_NAME, &timer.source, None, TimerExpired { name: timer.name });
        reply.body.in_reply_to = timer.msg_id;
        self.dispatch(None, Timestamp::now(), reply).await
    }

    /// handles a single core [`Message`] (i.e. [`Launch`] or [`BeginMonitor`]).
    /// Returns an error if the [`Message`] was not send from a client node, if the [`Message`]s type
    /// is not known, or if handling of the [`Message`] itself fails.
    async fn handle_core_message(&mut self, source: Option<(NodeId, MiddlewareId)>, timestamp: Timestamp, message: Message) -> Result<(), CoreError> {
        macro_rules! assert_is_client {
            () => {
                    if let Some((source_id, middleware_id)) = source {
                        if !self.nodes[source_id].is_client {
                        return Err(CoreError::IllegalCoreMessage {
                            source: self.nodes[source_id].commandline(middleware_id),
                            message,
                        });
                    }
                }
            }
        }

        match message.body.ty.as_str() {
            Next::TYPE => {
                return self.forward_to_next(source, message.payload::<Next>().unwrap().message);
            }
            libproto::services::Timer::TYPE => {
                let timer = message.payload::<libproto::services::Timer>().unwrap();
                if let Some((_, middleware_id)) = source {
                    self.timer_manager.add(Instant::now() + Duration::from_secs_f64(timer.seconds), message.body.msg_id, message.src, timer.name, middleware_id);
                }
                Ok(())
            }
            LogMessage::TYPE => {
                let message = message.payload::<LogMessage>().unwrap();
                let Some((source_id, middleware_id)) = source else {
                    panic!("tried to send log message without a source (i.e. the core sent it to the core?)");
                };
                self.log(timestamp, source_id, middleware_id, message).await
            }
            Break::TYPE => {
                if self.interactive {
                    self.state = CoreState::Paused;
                }
                Ok(())
            }
            Reset::TYPE => {
                assert_is_client!();
                self.reset = true;
                self.begin_shutdown(1..)?;
                Ok(())
            }
            Launch::TYPE => {
                assert_is_client!();
                self.launch_queue.push_back(message.payload::<Launch>().unwrap());
                Ok(())
            }
            BeginMonitor::TYPE => {
                assert_is_client!();
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
            Forward::TYPE => {
                // only the core can send messages of type "forward" to nodes, not the other way around
                let (source_id, middleware_id) = source.expect("core tried to send itself a `forward` message");
                Err(CoreError::IllegalCoreMessage {
                    source: self.nodes[source_id].commandline(middleware_id),
                    message,
                })
            }
            ty => {
                let (source_id, middleware_id) = source.expect("the core tried to send itself an unknown core message");
                Err(CoreError::UnknownCoreMessage { source: self.nodes[source_id].commandline(middleware_id), ty: ty.to_owned() })
            }
        }
    }

    /// forward a message to the next process below in the middleware stack
    fn forward_to_next(&mut self, source: Option<(NodeId, MiddlewareId)>, message: Message) -> Result<(), CoreError> {
        let (source_id, middleware_id) = source.expect("tried to send `next` type message from core");
        let node = &self.nodes[source_id];
        if node.has_middleware(middleware_id.below()) {
            node.send_to_middleware(ProcessCommand::Deliver(message), middleware_id.below());
            Ok(())
        } else {
            Err(CoreError::MissingMiddleware { source: node.commandline(middleware_id), node: node.name.clone(), middleware_id })
        }
    }

    /// signals al nodes to begin shutting down (e.g. close stdin handles etc.)
    fn begin_shutdown<R>(&mut self, range: R) -> Result<(), CoreError>
    where
        R: SliceIndex<[Node], Output=[Node]>,
    {
        for proc in &mut self.nodes[range] {
            proc.begin_shutdown();
        }
        Ok(())
    }

    /// launches a single new server
    async fn launch_single(&mut self, launch: Launch) -> Result<(), CoreError> {
        let error = if launch.as_client {
            if !launch.middleware_before.is_empty() || !launch.middleware_after.is_empty() {
                Some("cannot specify middleware when launching a client".to_string())
            } else {
                let node = self.nodes.push(self.nodes[0].alias(launch.name));
                self.protocol.publish_event(Event::node_launched(Timestamp::now(), node.id, node.name.clone(), node.commandline(MiddlewareId(0)))).await;
                None
            }
        } else {
            let node = self.launch_node_with_middleware(launch.name, &launch.middleware_before, &launch.middleware_after).await?;
            let id = node.id;
            let name = node.name.clone();
            let commandline = node.commandline(MiddlewareId(launch.middleware_before.len()));
            self.protocol.publish_event(Event::node_launched(Timestamp::now(), id, name, commandline)).await;
            None
        };
        self.dispatch(None, Timestamp::now(), Message::new(
            CORE_NAME,
            CLIENT_NAME,
            None,
            LaunchFinished { error },
        )).await?;
        Ok(())
    }

    /// Resets the [`Core`] and sets up a new test run
    async fn reset(&mut self) -> Result<(), CoreError> {
        self.nodes.drain(1..);
        self.monitor_sessions.clear();
        self.dispatch(None, Timestamp::now(), Message::new(CORE_NAME, CLIENT_NAME, None, ResetFinished {})).await?;
        self.protocol.publish_event(Event::reset(Timestamp::now())).await;
        Ok(())
    }

    /// launches a new node with its corresponding process and middleware processes
    async fn launch_node_with_middleware(&mut self, name: String, middleware_before: &[String], middleware_after: &[String]) -> Result<&mut Node, CoreError> {
        let node = self.launch(None, false, name.clone()).await?;
        let node_id = node.id;
        for middleware in middleware_before.iter().rev() {
            let middleware = self.launch_proc(Some(middleware), false, name.clone()).await?;
            self.nodes[node_id].push_middleware_before(middleware)
        }
        for middleware in middleware_after {
            let middleware = self.launch_proc(Some(middleware), false, name.clone()).await?;
            self.nodes[node_id].push_middleware_after(middleware)
        }
        self.dispatch(None, Timestamp::now(), Message::new(CORE_NAME, &name, None, Init {
            name: name.clone(),
        })).await?;
        Ok(&mut self.nodes[node_id])
    }

    /// launches a new process and creates the corresponding node
    async fn launch(&mut self, command: Option<&str>, is_client: bool, name: String) -> Result<&mut Node, CoreError> {
        let proc = self.launch_proc(command, is_client, name.clone()).await?;
        let node = self.nodes.push(Node::new(name, is_client, proc));
        let commandline = node.commandline(MiddlewareId(0));
        log::info!("[{}] command `{commandline}` launched", node.name);
        Ok(node)
    }

    async fn launch_proc(&mut self, command: Option<&str>, is_client: bool, name: String) -> Result<Process, CoreError> {
        let command = command.unwrap_or(&self.server_command);
        self.launcher.launch(command, is_client, name).await
            .map_err(|e| CoreError::LaunchFailed {
                command: command.to_string(),
                error: e,
            })
    }

    /// Sends a log event to all subscribers and writes the line to the current logger implementation.
    async fn log(&mut self, timestamp: Timestamp, source_id: NodeId, middleware_id: MiddlewareId, message: LogMessage) -> Result<(), CoreError> {
        let node = &self.nodes[source_id];
        if let Some(marker) = &message.marker {
            let (color, reset) = if let Some(color) = marker.color {
                (log_marker_ansi_color(color), log_color::RESET)
            } else {
                ("", "")
            };
            log::info!("[{}][{}]: {color}[{}] {}{reset}", node.commandline(middleware_id), node.name, marker.label, message.text);
        } else {
            log::info!("[{}][{}]: {}", node.commandline(middleware_id), node.name, message.text);
        }
        self.protocol.publish_event(Event::log(timestamp, source_id, message)).await;
        Ok(())
    }

    /// Sends a disconnect event to all subscribers and logs the exit code of the process.
    async fn process_exited(&mut self, timestamp: Timestamp, source_id: NodeId, middleware_id: MiddlewareId, exit_code: i32) -> Result<(), CoreError> {
        self.protocol.publish_event(Event::node_disconnected(timestamp, source_id)).await;
        let node = &self.nodes[source_id];
        log::info!("[{}] command `{}` exited with code {exit_code}", node.name, node.commandline(middleware_id));
        Ok(())
    }
}
