//! The core of the simulation, that governs the execution of the simulation
//!
//! The [`Core`] contains all logic to handle communication between nodes, handling of core [`Message`]s,
//! handling of [`RemoteCommand`]s and publishing of all [`Event`]s.
//!
//! As such it is also the point at which all [`ProcessEvent`]s are serialized into a definite order.
//! This order is itself non-deterministic and can change across multiple executions.
//!
//! When a [`Core`] is created, a single test process is launched. During execution the test process
//! can then send a number of [`Launch`] messages to the core, to give itself one or more (test-) node names, or launch
//! a number of server nodes, with given names. Each server node is then sent an [`Init`] message with its own name.
//!
//! After launching and initializing a node, the test process is sent a [`LaunchFinished`] message.
//!
//! When the test process sends a [`Reset`] message, the core closes all communications with
//! existing server processes and waits for them to exit.
//! It also clears all existing server and test node names, as well as all running [`MonitorSession`]s.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::ops::{Add, RangeBounds};
use std::slice::SliceIndex;

use async_channel::{Receiver, Sender};
use tokio::time::{Duration, Instant};

use libproto::init::Init;
use libproto::middleware::{Forward, Next};
use libproto::services::{LogMarker, LogMarkerColor, LogMessage, TimerExpired};
use libproto::system::{
    BeginMonitor, Break, Exited, Launch, LaunchFinished, MonitorEvent, MonitorEventKind, Register,
    Reset, ResetFinished,
};
use libproto::{Message, Payload};
use node::Node;

use crate::core::error::{CoreError, DispatchErrorKind};
use crate::core::event::Event;
use crate::core::monitor::MonitorSession;
use crate::core::node::{MiddlewareId, NodeId};
use crate::core::node_list::{NodeList, NodeRef};
use crate::core::remote_control::RemoteCommand;
use crate::core::timer_manager::{Timer, TimerKind, TimerManager};
use crate::log_color;
use crate::log_color::log_marker_ansi_color;
use crate::process::{Launcher, Process, ProcessCommand, ProcessEvent};
use crate::timestamp::{Timestamp, TimestampSource};
use crate::Command;
pub use builder::Builder;
use network::Network;

mod builder;
pub mod error;
pub mod event;
mod monitor;
mod network;
mod node;
mod node_list;
pub mod remote_control;
mod timer_manager;

/// The core of the simulation
///
/// This struct contains all state of the simulation and is used to drive execution forwards
/// by collecting [`ProcessEvent`]s from processes, delivering [`Message`]s and listening for
/// remote control commands.
pub struct Core {
    /// source for logical timestamps within a single run.
    /// Is automatically reset after a `reset` command is received
    timestamp_source: TimestampSource,
    /// Manages all nodes that are participating in the simulation
    nodes: NodeList,
    /// the [`NodeId`] of the test node (probably `NodeId(0)`) most of the time
    test_node_id: NodeId,
    /// the name of the test node
    test_node_name: String,
    /// the name of the simulation core
    /// this name must be used as the source/destination for "core" messages and is used in core logs
    core_name: String,
    /// launches new processes
    launcher: Launcher,
    /// Command string from which the test process was launched
    test_command: Command,
    /// Command string from which server processes are launched.
    server_command: Command,
    /// `true` if the program was started in interactive mode (i.e. with the user interface enabled)
    interactive: bool,
    /// The core expects each test to immediately send a `register` message
    /// to the core, so that it can detect if a server program has accidentally been
    /// started as a test program by the user, and can report accordingly.
    /// This flag can be used to suppress that behaviour.
    omit_test_register: bool,
    /// The current execution state (i.e. running/stepping/paused...)
    state: CoreState,
    /// Receives [`RemoteCommand`]s for controlling this [`Core`]
    remote_receiver: Receiver<RemoteCommand>,
    /// is cloned and given to whoever wants to remote control this [`Core`].
    remote_sender: Sender<RemoteCommand>,
    /// [`Event`]s are sent into this channel,
    event_sender: Sender<Event>,
    /// is cloned and given to whoever wants to subscribe to events from this [`Core`]
    event_receiver: Receiver<Event>,
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
    /// map of `NodeId`s that have exited and their exit code. When these nodes have all their
    /// outstanding messages delivered, the core sends an `"exited"` message to the test, if requested.
    exit_set: HashMap<NodeId, i32>,
    /// when the test sends a reset message, some stuff has to be finished/cleaned up before
    /// the new nodes may be launched,
    /// so we set this flag while we wait for everything to become ready
    reset_flag: Option<CoreReset>,
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

/// A flag used to shut down or reset the core
#[derive(Copy, Clone, Eq, PartialEq)]
enum CoreReset {
    Shutdown,
    Reset,
}

/// when launching a node, this is a convenient way to specify which command string should be used
#[derive(Copy, Clone)]
enum LaunchCommand<'a> {
    /// launch a new test process
    Test,
    /// launch a new server process
    Server,
    /// launch a custom command (e.g. for middleware)
    Custom(&'a Command),
}

impl From<Builder> for Core {
    fn from(builder: Builder) -> Self {
        let (remote_sender, remote_receiver) = async_channel::bounded(1);
        let (event_sender, event_receiver) = async_channel::unbounded();
        Self {
            timestamp_source: TimestampSource::new(),
            nodes: NodeList::new(),
            test_node_id: NodeId(0),
            test_node_name: builder.test_node_name,
            core_name: builder.core_name,
            launcher: Launcher::new(builder.allow_lua_unsafe),
            test_command: builder.test_command,
            server_command: builder.server_command,
            interactive: builder.interactive,
            omit_test_register: builder.omit_test_register,
            state: if builder.interactive {
                CoreState::Paused
            } else {
                CoreState::Running
            },
            remote_sender,
            remote_receiver,
            event_sender,
            event_receiver,
            monitor_sessions: Vec::new(),
            network: Network::new(),
            timer_manager: TimerManager::new(),
            launch_queue: VecDeque::new(),
            exit_set: HashMap::new(),
            reset_flag: None,
        }
    }
}

impl Core {
    /// returns a new [`Builder`] for configuring and building a new [`Core`]
    pub fn builder(test_command: Command, server_command: Command) -> Builder {
        Builder::new(test_command, server_command)
    }

    async fn restart(&mut self, re_init: bool) -> Result<(), CoreError> {
        if re_init {
            self.terminate_now(..).await;
            self.timestamp_source = TimestampSource::new();
            self.nodes = NodeList::new();
            self.state = if self.interactive {
                CoreState::Paused
            } else {
                CoreState::Running
            };
            self.monitor_sessions = Vec::new();
            self.network = Network::new();
            self.timer_manager = TimerManager::new();
            self.launch_queue = VecDeque::new();
            self.reset_flag = None;
        }

        if self.test_command.program != "" {
            // publish an initial "reset" event, so that the webapp can reset its state when "dsbox"
            // is re-started
            self.event_sender
                .send(Event::reset(self.timestamp_source.now()))
                .await
                .ok();

            self.test_node_id = self.launch_test_node().await?;
        }
        Ok(())
    }

    /// Returns a new [`Sender`] for sending [`RemoteCommand`]s to the [`Core`]
    pub fn remote_control(&self) -> Sender<RemoteCommand> {
        self.remote_sender.clone()
    }

    /// Returns a new [`ProtocolSubscriber`] for listening to events from the [`Core`]
    pub fn subscribe_events(&self) -> Receiver<Event> {
        self.event_receiver.clone()
    }

    /// starts the execution. This function consumes the passed [`Core`] because it cannot be restarted
    /// after [`Core::run`] returns.
    pub async fn run(mut self) {
        // launch test node/publish initial reset event
        if let Err(e) = self.restart(false).await {
            self.log_core_error(e).await;
            return;
        }
        let mut deadline = None;
        loop {
            let mut num_running = 0;
            let mut num_servers = 0;
            for node in self.nodes.iter_mut().filter_map(NodeRef::as_node_mut) {
                if node.has_finished() {
                    continue;
                }
                num_running += 1;
                if !node.is_test {
                    num_servers += 1;
                }
            }

            if !self.interactive && num_running == 0 {
                // in cli mode, finish automatically when all nodes have shut down
                break;
            }

            let dont_block = deadline.is_some() || !self.launch_queue.is_empty();

            if let Err(e) = self.step(dont_block).await {
                self.log_core_error(e).await;
            }
            if let Some(launch) = self.launch_queue.pop_front() {
                self.launch_single(launch).await;
            } else if self.reset_flag.is_some() && self.network.is_empty() {
                if deadline.is_none() {
                    deadline = Some(Instant::now() + Duration::from_secs(1));
                } else if num_servers == 0 || Instant::now() > deadline.unwrap() {
                    deadline = None;
                    let shutdown = self.reset_flag.take().unwrap() == CoreReset::Shutdown;
                    self.reset(shutdown).await;
                    if shutdown {
                        break;
                    };
                }
            }
        }
    }

    fn get_next_message_for_delivery(&mut self) -> Option<(Timestamp, Option<NodeId>, Message)> {
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

        if let Some((sent_timestamp, source_id, message)) = self.get_next_message_for_delivery() {
            self.deliver(sent_timestamp, source_id, message).await?;
            if self.state == CoreState::Stepping {
                self.state = CoreState::Paused;
            }
        } else {
            let timeout = async move {
                if dont_block {
                    tokio::time::sleep(Duration::from_millis(10)).await
                }
            };
            tokio::select! {
                biased;
                remote_command = self.remote_receiver.recv() => {
                    self.handle_command(remote_command.unwrap()).await?;
                }
                process_event = self.nodes.recv_any() => {
                    if let Some((event, node_id, middleware_idx)) = process_event {
                        let ts = self.timestamp_source.now();
                        self.handle_process_event(ts, node_id, middleware_idx, event).await?;
                    }
                }
                timer = self.timer_manager.wait_next() => {
                    self.handle_timer_expired(timer).await?;
                }
                _ = timeout, if dont_block => {}
            }
        }
        Ok(())
    }

    /// Handles a single [`ProcessEvent`].
    async fn handle_process_event(
        &mut self,
        timestamp: Timestamp,
        node_id: NodeId,
        middleware_id: MiddlewareId,
        process_event: ProcessEvent,
    ) -> Result<bool, CoreError> {
        log::trace!("handle_process_event: {:?}", process_event);
        match process_event {
            ProcessEvent::Message(message) => {
                self.dispatch(Some((node_id, middleware_id)), timestamp, message)
                    .await?;
                Ok(false)
            }
            ProcessEvent::Log(log) => {
                let log_message = LogMessage {
                    text: log,
                    marker: None,
                };
                self.log(timestamp, node_id, middleware_id, log_message)
                    .await;
                Ok(true)
            }
            ProcessEvent::Exited(exit_code) => {
                self.process_exited(timestamp, node_id, middleware_id, exit_code)
                    .await?;
                if self.nodes.resolve_alias(node_id).is_test {
                    // test process exited: shut down all processes gracefully
                    self.begin_shutdown(..);
                }
                Ok(true)
            }
            ProcessEvent::SerializeError { raw_message, error } => Err(CoreError::SerializeError {
                source: self.nodes.resolve_alias(node_id).commandline(middleware_id),
                raw_message,
                error,
            }),
        }
    }

    fn ensure_registered(&self, node_id: NodeId) -> Result<(), CoreError> {
        let node = self.nodes.resolve_alias(node_id);
        if node.requires_registration {
            Err(CoreError::MissingRegistration {
                source: node.name.clone(),
            })
        } else {
            Ok(())
        }
    }

    /// Dispatches a single [`Message`] into the network.
    async fn dispatch(
        &mut self,
        source: Option<(NodeId, MiddlewareId)>,
        timestamp: Timestamp,
        message: Message,
    ) -> Result<(), CoreError> {
        log::trace!("dispatching message {}", message.to_json());

        if let Some((source_id, _)) = source {
            let source = &self.nodes[source_id];
            if !self.nodes.alias_same_node(source, &message.src) {
                let aliases = self.nodes.aliases_of(source);
                let got = message.src.clone();
                return Err(CoreError::DispatchError {
                    name: source.name().to_owned(),
                    message,
                    kind: DispatchErrorKind::SourceNameMismatch(got, aliases),
                });
            }
        }

        self.event_sender
            .send(Event::send_message(timestamp, message.clone()))
            .await
            .ok();

        if message.dest == self.core_name {
            // handle messages to the core immediately, circumventing the network
            return self.handle_core_message(source, timestamp, message).await;
        }

        if let Some((source_id, middleware_id)) = source {
            let node = self.nodes.resolve_alias(source_id);
            if !middleware_id.is_top() {
                node.send_to_middleware(
                    ProcessCommand::Deliver(Message::new(
                        &self.core_name,
                        &node.name,
                        None,
                        Forward { message },
                    )),
                    middleware_id.above(),
                );
                return Ok(());
            }
        }

        self.send_monitor_event(timestamp, &message, None).await;
        if message.src == self.core_name {
            // deliver messages from the core immediately, circumventing the network
            let now = self.timestamp_source.now();
            self.deliver(now, source.map(|(id, _)| id), message).await?;
        } else {
            self.network.insert(timestamp, source.map(|s| s.0), message);
        }
        Ok(())
    }

    /// Delivers a single [`Message`] to the destination node.
    async fn deliver(
        &mut self,
        sent_timestamp: Timestamp,
        source_id: Option<NodeId>,
        message: Message,
    ) -> Result<(), CoreError> {
        log::trace!("deliver {message:?}");
        let result = if let Some(destination_id) =
            self.nodes.lookup_and_resolve(&message.dest).map(|n| n.id)
        {
            let timestamp = self.timestamp_source.now();
            self.send_monitor_event(timestamp, &message, Some(sent_timestamp.logical))
                .await;
            self.event_sender
                .send(Event::deliver_message(timestamp, sent_timestamp.logical))
                .await
                .ok();
            self.nodes
                .resolve_alias_mut(destination_id)
                .send(ProcessCommand::Deliver(message));
            Ok(())
        } else {
            Err(CoreError::DispatchError {
                name: message.src.clone(),
                message,
                kind: DispatchErrorKind::DestinationUnknown,
            })
        };
        if let Some(source_id) = source_id {
            self.maybe_notify_exited(source_id).await;
        }
        result
    }

    /// Checks whether any active monitoring session matches against the given [`Message`], and sends a [`MonitorEvent`]
    /// to the corresponding source node. If `in_reference_to` is `None`, the kind is [`MonitorEventKind::Sent`], otherwise
    /// it is [`MonitorEventKind::Delivered`].
    async fn send_monitor_event(
        &mut self,
        timestamp: Timestamp,
        message: &Message,
        in_reference_to: Option<usize>,
    ) {
        for session in &self.monitor_sessions {
            if session.matches(&message) {
                let monitor_node = self.nodes.lookup_and_resolve(session.source()).unwrap();
                // monitor events are not dispatched via the network. Instead, they are delivered directly
                // to the target node. Among other reasons, this de-clutters the message log (monitor events
                // should not be the target of any kind of debugging/visualization)
                monitor_node.send(ProcessCommand::Deliver(Message::new(
                    &self.core_name,
                    session.source(),
                    None,
                    MonitorEvent {
                        kind: if in_reference_to.is_some() {
                            MonitorEventKind::Delivered
                        } else {
                            MonitorEventKind::Sent
                        },
                        timestamp_logical: timestamp.logical,
                        timestamp_physical: timestamp.physical,
                        in_reference_to,
                        message: message.clone(),
                    },
                )));
            }
        }
    }

    async fn handle_timer_expired(&mut self, timer: Timer) -> Result<(), CoreError> {
        match timer.kind {
            TimerKind::TimerService {
                source,
                msg_id,
                name,
            } => self.send_timer_expired(source, msg_id, name).await,
            TimerKind::ExpectRegistry { node_id } => self.ensure_registered(node_id),
        }
    }

    async fn send_timer_expired(
        &mut self,
        source: String,
        msg_id: Option<usize>,
        name: String,
    ) -> Result<(), CoreError> {
        let mut reply = Message::new(&self.core_name, &source, None, TimerExpired { name });
        reply.body.in_reply_to = msg_id;
        let ts = self.timestamp_source.now();
        self.dispatch(None, ts, reply).await
    }

    /// handles a single core [`Message`] (i.e. [`Launch`] or [`BeginMonitor`]).
    /// Returns an error if the [`Message`] was not send from a test node, if the [`Message`]s type
    /// is not known, or if handling of the [`Message`] itself fails.
    async fn handle_core_message(
        &mut self,
        source: Option<(NodeId, MiddlewareId)>,
        timestamp: Timestamp,
        message: Message,
    ) -> Result<(), CoreError> {
        macro_rules! assert_is_test {
            () => {
                if let Some((source_id, middleware_id)) = source {
                    let source_node = self.nodes.resolve_alias(source_id);
                    if !source_node.is_test {
                        return Err(CoreError::IllegalCoreMessage {
                            source: source_node.commandline(middleware_id),
                            message,
                        });
                    }
                }
            };
        }

        match message.body.ty.as_str() {
            Next::TYPE => self.forward_to_next(source, message.payload::<Next>().unwrap().message),
            libproto::services::Timer::TYPE => {
                let timer = message.payload::<libproto::services::Timer>().unwrap();
                self.timer_manager.add(
                    Instant::now() + Duration::from_secs_f64(timer.seconds),
                    TimerKind::TimerService {
                        msg_id: message.body.id,
                        source: message.src,
                        name: timer.name,
                    },
                );
                Ok(())
            }
            LogMessage::TYPE => {
                let message = message.payload::<LogMessage>().unwrap();
                let Some((source_id, middleware_id)) = source else {
                    panic!(
                        "tried to send log message without a source (i.e. the core sent it to the core?)"
                    );
                };
                self.log(timestamp, source_id, middleware_id, message).await;
                Ok(())
            }
            Break::TYPE => {
                if self.interactive {
                    self.state = CoreState::Paused;
                }
                Ok(())
            }
            Register::TYPE => {
                if let Some((source_id, _)) = source {
                    let node = self.nodes.resolve_alias_mut(source_id);
                    if !node.requires_registration {
                        return Err(CoreError::UnexpectedRegistration {
                            source: node.name.clone(),
                        });
                    } else {
                        node.requires_registration = false;
                    }
                }
                Ok(())
            }
            Reset::TYPE => {
                assert_is_test!();
                if self.reset_flag.is_none() {
                    self.reset_flag = Some(CoreReset::Reset);
                }
                self.begin_shutdown(1..);
                Ok(())
            }
            Launch::TYPE => {
                assert_is_test!();
                self.launch_queue
                    .push_back(message.payload::<Launch>().unwrap());
                Ok(())
            }
            BeginMonitor::TYPE => {
                assert_is_test!();
                let begin_monitor = message.payload::<BeginMonitor>().unwrap();
                let session = match MonitorSession::new(
                    message.src,
                    &begin_monitor.src_match,
                    &begin_monitor.dst_match,
                ) {
                    Ok(session) => session,
                    Err(err) => {
                        log::warn!(
                            "failed to start monitor session, source or destination expression invalid: {err}"
                        );
                        return Ok(());
                    }
                };
                self.monitor_sessions.push(session);
                Ok(())
            }
            Forward::TYPE => {
                // only the core can send messages of type "forward" to nodes, not the other way around
                let (source_id, middleware_id) =
                    source.expect("core tried to send itself a `forward` message");
                Err(CoreError::IllegalCoreMessage {
                    source: self
                        .nodes
                        .resolve_alias(source_id)
                        .commandline(middleware_id),
                    message,
                })
            }
            ty => {
                let (source_id, middleware_id) =
                    source.expect("the core tried to send itself an unknown core message");
                Err(CoreError::UnknownCoreMessage {
                    source: self
                        .nodes
                        .resolve_alias(source_id)
                        .commandline(middleware_id),
                    ty: ty.to_owned(),
                })
            }
        }
    }

    /// forward a message to the next process below in the middleware stack
    fn forward_to_next(
        &mut self,
        source: Option<(NodeId, MiddlewareId)>,
        message: Message,
    ) -> Result<(), CoreError> {
        let (source_id, middleware_id) =
            source.expect("tried to send `next` type message from core");
        let node = &self.nodes.resolve_alias(source_id);
        if node.has_middleware(middleware_id.below()) {
            node.send_to_middleware(ProcessCommand::Deliver(message), middleware_id.below());
            Ok(())
        } else {
            Err(CoreError::MissingMiddleware {
                source: node.commandline(middleware_id),
                node: node.name.clone(),
                middleware_id,
            })
        }
    }

    /// signals all nodes to begin shutting down (e.g. close stdin handles etc.)
    fn begin_shutdown<R>(&mut self, range: R)
    where
        R: SliceIndex<[NodeRef], Output = [NodeRef]>,
    {
        for proc in &mut self.nodes[range] {
            if let Some(proc) = proc.as_node_mut() {
                proc.begin_shutdown();
            }
        }
    }

    /// begin shutdown of nodes in given `range` and wait a grace period of 1 second before returning
    async fn terminate_now<R>(&mut self, range: R)
    where
        R: RangeBounds<usize>,
    {
        let deadline = Instant::now() + Duration::from_secs(1);
        let shutdowns = self
            .nodes
            .drain(range)
            .filter_map(NodeRef::into_node)
            .map(|node| node.terminate());
        tokio::time::timeout_at(deadline, futures::future::join_all(shutdowns))
            .await
            .ok();
    }

    /// launches a single new server
    async fn launch_single(&mut self, launch: Launch) {
        let error = if launch.as_test {
            if !launch.middleware_before.is_empty() || !launch.middleware_after.is_empty() {
                Some("cannot specify middleware when launching a test node".to_string())
            } else {
                let (alias_id, node) = self.nodes.add_alias(self.test_node_id, launch.name.clone());
                self.event_sender
                    .send(Event::node_launched(
                        self.timestamp_source.now(),
                        alias_id,
                        launch.name.clone(),
                        node.commandline(MiddlewareId(0)),
                    ))
                    .await
                    .ok();
                None
            }
        } else {
            let launch_result = self
                .launch_node_with_middleware(
                    launch.name,
                    launch.request_exited_message,
                    &launch.middleware_before,
                    &launch.middleware_after,
                )
                .await;
            match launch_result {
                Ok(node) => {
                    let id = node.id;
                    let name = node.name.clone();
                    let commandline =
                        node.commandline(MiddlewareId(launch.middleware_before.len()));
                    self.event_sender
                        .send(Event::node_launched(
                            self.timestamp_source.now(),
                            id,
                            name,
                            commandline,
                        ))
                        .await
                        .ok();
                    None
                }
                Err(e) => Some(e.to_string()),
            }
        };
        let ts = self.timestamp_source.now();
        self.dispatch(
            None,
            ts,
            Message::new(
                &self.core_name,
                &self.test_node_name,
                None,
                LaunchFinished { error },
            ),
        )
        .await
        .expect("sending launch_finished message");
    }

    /// Resets the [`Core`] and sets up a new test run
    async fn reset(&mut self, shutdown: bool) {
        if shutdown {
            self.terminate_now(0..).await;
        } else {
            self.terminate_now(1..).await;
        }

        self.monitor_sessions.clear();
        if !shutdown {
            // send the "ResetFinished" event to the test node with the old timestamp source
            let ts = self.timestamp_source.now();
            self.dispatch(
                None,
                ts,
                Message::new(
                    &self.core_name,
                    &self.test_node_name,
                    None,
                    ResetFinished {},
                ),
            )
            .await
            .expect("sending reset_finished message");

            // reset the timestamps to restart at 0
            self.timestamp_source = TimestampSource::new();

            self.event_sender
                .send(Event::reset(self.timestamp_source.now()))
                .await
                .ok();
        }
    }

    /// launches a new node with its corresponding process and middleware processes
    async fn launch_node_with_middleware(
        &mut self,
        name: String,
        exited_message_requested: bool,
        middleware_before: &[Command],
        middleware_after: &[Command],
    ) -> Result<&mut Node, CoreError> {
        let node = self
            .launch(
                LaunchCommand::Server,
                false,
                exited_message_requested,
                name.clone(),
            )
            .await?;
        let node_id = node.id;
        for middleware in middleware_before.iter().rev() {
            let middleware = self
                .launch_proc(LaunchCommand::Custom(middleware), false, name.clone())
                .await?;
            self.nodes
                .resolve_alias_mut(node_id)
                .push_middleware_before(middleware)
        }
        for middleware in middleware_after {
            let middleware = self
                .launch_proc(LaunchCommand::Custom(middleware), false, name.clone())
                .await?;
            self.nodes
                .resolve_alias_mut(node_id)
                .push_middleware_after(middleware)
        }
        let ts = self.timestamp_source.now();
        self.dispatch(
            None,
            ts,
            Message::new(
                &self.core_name,
                &name,
                None,
                Init {
                    name: name.clone(),
                    core_name: self.core_name.clone(),
                    is_test: false,
                },
            ),
        )
        .await?;
        Ok(self.nodes.resolve_alias_mut(node_id))
    }

    async fn launch_test_node(&mut self) -> Result<NodeId, CoreError> {
        let node_id = self
            .launch(
                LaunchCommand::Test,
                true,
                false,
                self.test_node_name.clone(),
            )
            .await?
            .id;
        let node = self.nodes.resolve_alias(node_id);
        let commandline = node.commandline(MiddlewareId(0));
        self.event_sender
            .send(Event::node_launched(
                self.timestamp_source.now(),
                node_id,
                self.test_node_name.clone(),
                commandline,
            ))
            .await
            .ok();

        let timestamp = self.timestamp_source.now();
        self.dispatch(
            None,
            timestamp,
            Message::new(
                &self.core_name,
                &self.test_node_name,
                None,
                Init {
                    name: self.test_node_name.clone(),
                    core_name: self.core_name.clone(),
                    is_test: true,
                },
            ),
        )
        .await?;
        Ok(node_id)
    }

    /// launches a new process and creates the corresponding node
    async fn launch(
        &mut self,
        command: LaunchCommand<'_>,
        is_test: bool,
        exited_message_requested: bool,
        name: String,
    ) -> Result<&mut Node, CoreError> {
        let proc = self.launch_proc(command, is_test, name.clone()).await?;
        let requires_registration = is_test && !self.omit_test_register;
        let node = self.nodes.add(Node::new(
            name,
            is_test,
            exited_message_requested,
            requires_registration,
            proc,
        ));
        let commandline = node.commandline(MiddlewareId(0));
        log::info!("[{}] command `{commandline}` launched", node.name);
        if requires_registration {
            self.timer_manager.add(
                Instant::now().add(Duration::from_millis(500)),
                TimerKind::ExpectRegistry { node_id: node.id },
            )
        }
        Ok(node)
    }

    async fn launch_proc(
        &mut self,
        command: LaunchCommand<'_>,
        is_test: bool,
        name: String,
    ) -> Result<Process, CoreError> {
        let command = match command {
            LaunchCommand::Test => &self.test_command,
            LaunchCommand::Server => &self.server_command,
            LaunchCommand::Custom(command) => command,
        };
        self.launcher
            .launch(command.clone(), is_test, name, self.core_name.clone())
            .await
            .map_err(|e| CoreError::LaunchFailed {
                command: command.to_string(),
                error: e,
            })
    }

    /// Sends a log event to all subscribers and writes the line to the current logger implementation.
    async fn log(
        &mut self,
        timestamp: Timestamp,
        source_id: NodeId,
        middleware_id: MiddlewareId,
        message: LogMessage,
    ) {
        let node = self.nodes.resolve_alias(source_id);
        if let Some(marker) = &message.marker {
            let (color, reset) = if let Some(color) = marker.color {
                (log_marker_ansi_color(color), log_color::RESET)
            } else {
                ("", "")
            };
            log::info!(
                "[{}][{}]: {color}[{}] {}{reset}",
                node.commandline(middleware_id),
                node.name,
                marker.label,
                message.text
            );
        } else {
            log::info!(
                "[{}][{}]: {}",
                node.commandline(middleware_id),
                node.name,
                message.text
            );
        }
        self.event_sender
            .send(Event::log(timestamp, source_id, message))
            .await
            .ok();
    }

    async fn log_core_error(&mut self, error: CoreError) {
        let message = format!("simulation core error:\n{error}");
        log::error!("{message}");
        self.event_sender
            .send(Event::log(
                self.timestamp_source.now(),
                self.test_node_id,
                LogMessage {
                    text: message,
                    marker: Some(LogMarker {
                        label: "ERR".to_string(),
                        color: Some(LogMarkerColor::Red),
                    }),
                },
            ))
            .await
            .ok();
    }

    /// Sends a disconnect event to all subscribers and logs the exit code of the process.
    ///
    /// Additionally, if there are no further messages in the network for this node, notifies the
    /// test node that the node has exited (if requested). Otherwise, saves the exit code for later
    /// delivery of the exit notification.
    async fn process_exited(
        &mut self,
        timestamp: Timestamp,
        source_id: NodeId,
        middleware_id: MiddlewareId,
        exit_code: i32,
    ) -> Result<(), CoreError> {
        self.event_sender
            .send(Event::node_disconnected(timestamp, source_id))
            .await
            .ok();
        let node = self.nodes.resolve_alias(source_id);
        log::info!(
            "[{}] command `{}` exited with code {exit_code}",
            node.name,
            node.commandline(middleware_id)
        );

        self.exit_set.insert(source_id, exit_code);
        self.maybe_notify_exited(source_id).await;
        Ok(())
    }

    /// checks if the given node has all remaining message delivered and sends an
    /// exit notification to the test node
    async fn maybe_notify_exited(&mut self, node_id: NodeId) {
        let exit_code = match self.exit_set.entry(node_id) {
            Entry::Occupied(entry) => {
                if self.network.has_remaining_messages(node_id) {
                    return;
                }
                entry.remove()
            }
            Entry::Vacant(_) => return,
        };

        let node = self.nodes.resolve_alias(node_id);
        if node.exited_message_requested {
            let timestamp = self.timestamp_source.now();
            // box the future to because of recursing in an async function
            let future = self.dispatch(
                None,
                timestamp,
                Message::new(
                    &self.core_name,
                    &self.test_node_name,
                    None,
                    Exited {
                        name: node.name.clone(),
                        exit_code,
                    },
                ),
            );
            Box::pin(future).await.unwrap();
        }
    }

    /// split a string into the program and args
    /// for now, it just splits the string using the space character,
    /// taking the first element as the program and the remaining elements as the args
    pub fn split_command(command: impl AsRef<str>) -> Command {
        Self::make_command(command.as_ref().split(" ").map(|s| s.to_string()))
    }

    /// make a command from an iterator of strings. The first element becomes the program,
    /// the remaining elements become the args
    pub fn make_command(command: impl IntoIterator<Item = String>) -> Command {
        let mut command = command.into_iter();
        let program = command.next().unwrap_or_default();
        let args = command.collect::<Vec<_>>();
        Command { program, args }
    }
}
