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

use std::collections::HashMap;
use std::ops::Add;

use async_channel::{Receiver, Sender};
use enumflags2::BitFlags;
use tokio::time::{Duration, Instant};

use libproto::init::Init;
use libproto::services::{LogMarker, LogMarkerColor, LogMessage, TimerExpired};
use libproto::system::{
    Alias, BeginMonitor, Exited, Launch, LaunchFinished, MonitorEvent, MonitorEventKind, Register,
    Reset, ResetFinished,
};
use libproto::{Message, Payload};
use node::Node;

use crate::command::ExecutableCommand;
use crate::capabilities::Capability;
use crate::core::error::{CoreError, DispatchErrorKind};
use crate::core::event::Event;
use crate::core::monitor::MonitorSession;
use crate::core::node::NodeId;
use crate::core::node_manager::NodeManager;
use crate::core::remote_control::RemoteCommand;
use crate::core::timer_manager::{Timer, TimerKind, TimerManager};
use crate::log_color;
use crate::log_color::log_marker_ansi_color;
use crate::process::{Process, ProcessCommand, ProcessEvent};
use crate::timestamp::{Timestamp, TimestampSource};
pub use builder::Builder;
use libproto::system::control::{Break, Control, SubscribeEvents};
use network::Network;
use crate::process::launcher::Launcher;

mod builder;
pub mod error;
pub mod event;
mod monitor;
mod network;
mod node;
mod node_manager;
pub mod remote_control;
mod timer_manager;
mod version;

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
    nodes: NodeManager,
    /// the [`NodeId`] of the test node
    test_node_id: NodeId,
    /// the name of the test node
    test_node_name: String,
    /// the exit code of the test. Will be returned from [`Core::run`], and in cli mode
    /// determines the return code of the whole program. Useful for automated testing.
    test_exit_code: i32,
    /// the name of the simulation core
    /// this name must be used as the source/destination for "core" messages and is used in core logs
    core_name: String,
    /// launches new processes
    launcher: Launcher,
    /// registry of commands, that can be launched through [`Launch`] Messages, and their capabilities
    commands: HashMap<String, (ExecutableCommand, BitFlags<Capability>)>,
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


impl From<Builder> for Core {
    fn from(builder: Builder) -> Self {
        let (remote_sender, remote_receiver) = async_channel::bounded(1);
        let (event_sender, event_receiver) = async_channel::unbounded();
        Self {
            timestamp_source: TimestampSource::new(),
            nodes: NodeManager::new(),
            // TODO: Core::test_node_id should not be needed anymore in the future
            test_node_id: unsafe { std::mem::transmute(0usize) },
            test_node_name: builder.test_node_name,
            test_exit_code: 0,
            core_name: builder.core_name,
            launcher: Launcher::new(builder.allow_lua_unsafe),
            commands: HashMap::from([
                (
                    "test".to_owned(),
                    (
                        builder.test_command,
                        BitFlags::<Capability>::default()
                            | Capability::LaunchNodes
                            | Capability::LaunchAlias
                            | Capability::Monitor
                            | Capability::Reset,
                    ),
                ),
                (
                    "server".to_owned(),
                    (builder.server_command, BitFlags::<Capability>::default()),
                ),
            ]),
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
        }
    }
}

impl Core {
    /// returns a new [`Builder`] for configuring and building a new [`Core`]
    pub fn builder(test_command: ExecutableCommand, server_command: ExecutableCommand) -> Builder {
        Builder::new(test_command, server_command)
    }

    async fn setup(&mut self) -> Result<(), CoreError> {
        if self.commands.get("test").unwrap().0.program != "" {
            // publish an initial "reset" event, so that the webapp can reset its state when "dsbox"
            // is re-started
            self.event_sender
                .send(Event::reset(self.timestamp_source.now()))
                .await
                .ok();

            self.test_node_id = self.launch_test_node(self.test_node_name.clone()).await?;
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
    pub async fn run(mut self) -> i32 {
        // launch test node/publish initial reset event
        if let Err(e) = self.setup().await {
            self.log_core_error(e).await;
            return -1;
        }
        loop {
            let num_running = self.nodes.iter().filter(|n| !n.has_finished()).count();

            if num_running == 0 && self.network.is_empty() {
                // finish automatically when all nodes have shut down and all messages
                // have been delivered
                break;
            }

            if let Err(e) = self.step(num_running).await {
                self.log_core_error(e).await;
            }
        }
        self.test_exit_code
    }

    fn get_next_message_for_delivery(&mut self) -> Option<(Timestamp, Option<NodeId>, Message)> {
        if self.state != CoreState::Paused {
            self.network.remove_oldest()
        } else {
            None
        }
    }

    async fn step(&mut self, num_running: usize) -> Result<(), CoreError> {
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
            tokio::select! {
                biased;
                remote_command = self.remote_receiver.recv() => {
                    self.handle_command(remote_command.unwrap()).await?;
                }
                process_event = self.nodes.recv_any(), if num_running > 0 => {
                    if let Some((event, node_id)) = process_event {
                        let ts = self.timestamp_source.now();
                        self.handle_process_event(ts, node_id, event).await?;
                    }
                }
                timer = self.timer_manager.wait_next() => {
                    self.handle_timer_expired(timer).await?;
                }
            }
        }
        Ok(())
    }

    /// Handles a single [`ProcessEvent`].
    async fn handle_process_event(
        &mut self,
        timestamp: Timestamp,
        source_id: NodeId,
        process_event: ProcessEvent,
    ) -> Result<bool, CoreError> {
        log::trace!("handle_process_event: {:?}", process_event);
        match process_event {
            ProcessEvent::Message(message) => {
                self.dispatch(Some(source_id), timestamp, message).await?;
                Ok(false)
            }
            ProcessEvent::Log(log) => {
                let log_message = LogMessage {
                    text: log,
                    marker: None,
                };
                self.log(timestamp, source_id, None, log_message).await;
                Ok(true)
            }
            ProcessEvent::Exited(exit_code) => {
                self.process_exited(timestamp, source_id, exit_code).await?;
                Ok(true)
            }
            ProcessEvent::SerializeError { raw_message, error } => Err(CoreError::SerializeError {
                source: self.nodes[source_id].commandline(),
                raw_message,
                error,
            }),
        }
    }

    fn ensure_registered(&self, node_id: NodeId) -> Result<(), CoreError> {
        let node = &self.nodes[node_id];
        if node.requires_registration {
            Err(CoreError::MissingRegistration {
                name: node.name.clone(),
            })
        } else {
            Ok(())
        }
    }

    /// Dispatches a single [`Message`] into the network.
    async fn dispatch(
        &mut self,
        mut source: Option<NodeId>,
        timestamp: Timestamp,
        message: Message,
    ) -> Result<(), CoreError> {
        log::trace!("dispatching message {}", message.to_json());

        if let Some(source_id) = source {
            if !self.nodes.has_alias(source_id, &message.src) {
                let aliases = self.nodes.aliases_of(source_id);
                let got = message.src.clone();
                return Err(CoreError::DispatchError {
                    name: self.nodes[source_id].name.clone(),
                    message,
                    kind: DispatchErrorKind::SourceNameMismatch(got, aliases),
                });
            }
            // `source` up to this point is the NodeId of the Process which
            // sent the message, but if the was sent from an alias of that process
            // we figure out the actual node id of the messages source here
            source = self.nodes.lookup(&message.src);
        }

        self.event_sender
            .send(Event::send_message(timestamp, message.clone()))
            .await
            .ok();

        if message.dest == self.core_name {
            // handle messages to the core immediately, circumventing the network
            return self.handle_core_message(source, timestamp, message).await;
        }

        self.send_monitor_event(timestamp, &message, None).await;
        if message.src == self.core_name {
            // deliver messages from the core immediately, circumventing the network
            let now = self.timestamp_source.now();
            self.deliver(now, source.map(|id| id), message).await?;
        } else {
            self.network.insert(timestamp, source, message);
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
        let result = if let Some(destination_id) = self.nodes.lookup(&message.dest) {
            let timestamp = self.timestamp_source.now();
            self.send_monitor_event(timestamp, &message, Some(sent_timestamp.logical))
                .await;
            self.event_sender
                .send(Event::deliver_message(timestamp, sent_timestamp.logical))
                .await
                .ok();
            self.nodes[destination_id].send(ProcessCommand::Deliver(message));
            Ok(())
        } else {
            Err(CoreError::DispatchError {
                name: message.src.clone(),
                message,
                kind: DispatchErrorKind::DestinationUnknown,
            })
        };
        if let Some(source_id) = source_id {
            self.check_exited(source_id).await;
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
                // TODO: only deliver monitor messages if the message source and destination node
                //       was launched by the sessions source node?
                let monitor_node = self.nodes.lookup(session.source()).unwrap();
                let monitor_node = &self.nodes[monitor_node];
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
        source: Option<NodeId>,
        timestamp: Timestamp,
        message: Message,
    ) -> Result<(), CoreError> {
        macro_rules! assert_has_capability {
            ($msg_ty:path) => {
                if let Some(source_id) = source {
                    let source_node = &self.nodes[source_id];
                    if !source_node.has_capability(<$msg_ty as Payload>::TYPE) {
                        return Err(CoreError::IllegalCoreMessage {
                            name: message.src.clone(),
                            message,
                        });
                    }
                }
            };
        }

        match message.body.ty.as_str() {
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
                let source_name = message.src.clone();
                let message = message.payload::<LogMessage>().unwrap();
                let Some(source_id) = source else {
                    panic!(
                        "tried to send log message without a source (i.e. the core sent it to the core?)"
                    );
                };
                self.log(timestamp, source_id, Some(source_name), message)
                    .await;
                Ok(())
            }
            Register::TYPE => {
                if let Some(source_id) = source {
                    let node = &mut self.nodes[source_id];
                    if !node.requires_registration {
                        return Err(CoreError::UnexpectedRegistration {
                            name: node.name.clone(),
                        });
                    } else {
                        node.requires_registration = false;
                    }
                }
                Ok(())
            }
            Reset::TYPE => {
                assert_has_capability!(Reset);
                let source_id = source.expect("the core tried to send itself a reset message");
                self.terminate(|n| n.launched_by == Some(source_id)).await;
                let removed_aliases = self.nodes.remove_aliases_of(source_id);
                self.cleanup_aliases(removed_aliases);
                let node = &self.nodes[source_id];
                node.send(ProcessCommand::Deliver(Message::new(
                    &self.core_name,
                    &node.name,
                    None,
                    ResetFinished {},
                )));
                Ok(())
            }
            Launch::TYPE => {
                assert_has_capability!(Launch);
                let source_id = source.expect("the core tried to send itself a launch message");
                // pin the future here to deal with recursion (launch_single calls dispatch)
                Box::pin(self.launch_single(message.payload::<Launch>().unwrap(), source_id)).await;
                Ok(())
            }
            Alias::TYPE => {
                assert_has_capability!(Alias);
                let source_id = source.expect("the core tried to send itself an alias message");
                // pin the future here to deal with recursion (create_alias calls dispatch)
                Box::pin(self.create_alias(source_id, message.payload::<Alias>().unwrap())).await;
                Ok(())
            }
            BeginMonitor::TYPE => {
                assert_has_capability!(BeginMonitor);
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
            Break::TYPE => {
                assert_has_capability!(Break);
                if self.interactive {
                    self.state = CoreState::Paused;
                }
                Ok(())
            }
            Control::TYPE => {
                assert_has_capability!(Control);
                todo!()
            }
            SubscribeEvents::TYPE => {
                assert_has_capability!(SubscribeEvents);
                todo!()
            }
            ty => Err(CoreError::UnknownCoreMessage {
                name: message.src,
                ty: ty.to_owned(),
            }),
        }
    }

    /// shutdown the nodes that match the given predicate and wait a grace period of 1 second before
    /// removing the node and collecting all garbage (i.e. outstanding messages, aliases, etc.)
    async fn terminate<P>(&mut self, predicate: P)
    where
        P: FnMut(&&mut Node) -> bool,
    {
        let deadline = Instant::now() + Duration::from_secs(1);
        let nodes = self.nodes.iter_mut().filter(predicate).collect::<Vec<_>>();
        let node_ids = nodes.iter().map(|n| n.id).collect::<Vec<_>>();
        let shutdowns = nodes.into_iter().map(|node| node.terminate());

        tokio::time::timeout_at(deadline, futures::future::join_all(shutdowns))
            .await
            .ok();

        for node_id in node_ids {
            self.cleanup_node(node_id)
        }
    }

    fn cleanup_node(&mut self, cleanup_id: NodeId) {
        self.cleanup_aliases(self.nodes.aliases_of(cleanup_id));

        // remove the node and all its aliases
        self.nodes.remove(cleanup_id);

        // remove all active timers for that node
        self.timer_manager.retain(|timer| match &timer.kind {
            TimerKind::TimerService { .. } => true,
            TimerKind::ExpectRegistry { node_id } => *node_id != cleanup_id,
        })
    }

    fn cleanup_aliases<S: AsRef<str>>(&mut self, aliases: impl AsRef<[S]>) {
        let aliases = aliases.as_ref();
        // remove (drop) all messages from and to the removed node
        self.network.retain(|msg| {
            aliases.iter().all(|alias| {
                let alias = alias.as_ref();
                alias != msg.src && alias != msg.dest
            })
        });

        // remove all monitor sessions active for this node
        self.monitor_sessions
            .retain(|s| aliases.iter().all(|alias| alias.as_ref() != s.source()));

        // remove all active timers for that node
        self.timer_manager.retain(|timer| match &timer.kind {
            TimerKind::TimerService { name, .. } => {
                aliases.iter().all(|alias| alias.as_ref() != name)
            }
            TimerKind::ExpectRegistry { .. } => true,
        })
    }

    /// creates an alias for a server
    async fn create_alias(&mut self, for_id: NodeId, alias: Alias) {
        let error = match self.nodes.add_alias(for_id, alias.name.clone()) {
            Ok(Some(node)) => {
                self.event_sender
                    .send(Event::node_launched(
                        self.timestamp_source.now(),
                        alias.name,
                        node.commandline(),
                    ))
                    .await
                    .ok();
                None
            }
            Ok(None) => None,
            Err(_) => Some(CoreError::DuplicateNodeName { name: alias.name }.to_string()),
        };
        let ts = self.timestamp_source.now();
        self.dispatch(
            None,
            ts,
            Message::new(
                &self.core_name,
                &self.nodes[for_id].name,
                None,
                LaunchFinished { error },
            ),
        )
        .await
        .expect("sending launch_finished message");
    }

    /// launches a single new server
    async fn launch_single(&mut self, launch: Launch, launched_by: NodeId) {
        let launch_result = self
            .launch_server_node(
                launch.name,
                launch.command_name,
                launched_by,
                launch.request_exited_message,
            )
            .await;
        let error = match launch_result {
            Ok(node) => {
                let name = node.name.clone();
                let commandline = node.commandline();
                self.event_sender
                    .send(Event::node_launched(
                        self.timestamp_source.now(),
                        name,
                        commandline,
                    ))
                    .await
                    .ok();
                None
            }
            Err(e) => Some(e.to_string()),
        };
        let ts = self.timestamp_source.now();
        self.dispatch(
            None,
            ts,
            Message::new(
                &self.core_name,
                &self.nodes[launched_by].name,
                None,
                LaunchFinished { error },
            ),
        )
        .await
        .expect("sending launch_finished message");
    }

    /// launches a new node with its corresponding process and middleware processes
    async fn launch_server_node(
        &mut self,
        name: String,
        command_name: String,
        launched_by: NodeId,
        exited_message_requested: bool,
    ) -> Result<&mut Node, CoreError> {
        let node = self
            .launch(
                &command_name,
                Some(launched_by),
                false,
                exited_message_requested,
                name.clone(),
            )
            .await?;
        let node_id = node.id;
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
                    core_version: version::current(),
                    is_test: false,
                },
            ),
        )
        .await?;
        Ok(&mut self.nodes[node_id])
    }

    async fn launch_test_node(&mut self, name: impl Into<String>) -> Result<NodeId, CoreError> {
        let name = name.into();
        let node_id = self
            .launch("test", None, !self.omit_test_register, false, name.clone())
            .await?
            .id;
        let node = &self.nodes[node_id];
        let commandline = node.commandline();
        self.event_sender
            .send(Event::node_launched(
                self.timestamp_source.now(),
                name.clone(),
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
                &name,
                None,
                Init {
                    name: name.clone(),
                    core_name: self.core_name.clone(),
                    core_version: version::current(),
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
        command_name: &str,
        launched_by: Option<NodeId>,
        requires_registration: bool,
        exited_message_requested: bool,
        name: String,
    ) -> Result<&mut Node, CoreError> {
        let (command, capabilities) = self
            .commands
            .get(command_name)
            .ok_or_else(|| CoreError::UnknownCommand {
                command_name: command_name.to_string(),
            })?
            .clone();
        let proc = self.launch_proc(command, name.clone()).await?;
        let node = self
            .nodes
            .add(Node::new(
                name.clone(),
                launched_by,
                capabilities,
                exited_message_requested,
                requires_registration,
                proc,
            ))
            .map_err(|_| CoreError::DuplicateNodeName { name })?;
        let commandline = node.commandline();
        log::info!("[{}] command `{commandline}` launched", node.name);
        if requires_registration {
            self.timer_manager.add(
                Instant::now().add(Duration::from_millis(500)),
                TimerKind::ExpectRegistry { node_id: node.id },
            )
        }
        Ok(node)
    }

    async fn launch_proc(&mut self, command: ExecutableCommand, name: String) -> Result<Process, CoreError> {
        self.launcher
            .launch(command.clone(), name, self.core_name.clone())
            .await
            .map_err(|e| CoreError::LaunchFailed {
                command: command.to_string(),
                error: e,
            })
    }

    /// Sends a log event to all subscribers and writes the line to the current logger implementation.
    async fn log(
        &self,
        timestamp: Timestamp,
        source_id: NodeId,
        override_name: Option<String>,
        message: LogMessage,
    ) {
        let node = &self.nodes[source_id];
        let source_name = override_name.unwrap_or_else(|| node.name.clone());
        if let Some(marker) = &message.marker {
            let (color, reset) = if let Some(color) = marker.color {
                (log_marker_ansi_color(color), log_color::RESET)
            } else {
                ("", "")
            };
            log::info!(
                "[{}][{}]: {color}[{}] {}{reset}",
                source_name,
                node.name.clone(),
                marker.label,
                message.text
            );
        } else {
            log::info!("[{}][{}]: {}", source_name, node.name.clone(), message.text);
        }
        self.event_sender
            .send(Event::log(timestamp, node.name.clone(), message))
            .await
            .ok();
    }

    async fn log_core_error(&mut self, error: CoreError) {
        let message = format!("simulation core error:\n{error}");
        log::error!("{message}");
        self.event_sender
            .send(Event::log(
                self.timestamp_source.now(),
                self.core_name.clone(),
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
        exit_code: i32,
    ) -> Result<(), CoreError> {
        // shutdown all nodes that were launched by this node
        for node in self
            .nodes
            .iter_mut()
            .filter(|node| node.launched_by == Some(source_id))
        {
            node.begin_shutdown();
        }

        let node = &self.nodes[source_id];
        self.event_sender
            .send(Event::node_disconnected(timestamp, node.name.clone()))
            .await
            .ok();
        log::info!(
            "[{}] command `{}` exited with code {exit_code}",
            node.name,
            node.commandline()
        );

        if source_id == self.test_node_id {
            self.test_exit_code = exit_code;
        }

        self.check_exited(source_id).await;
        Ok(())
    }

    /// checks if the given node has exited and all remaining message delivered, sends an exit
    /// notification to the test node, if requested, and collects garbage for that node
    async fn check_exited(&mut self, node_id: NodeId) {
        let node = &mut self.nodes[node_id];
        if !node.has_finished() {
            return;
        }
        if self.network.has_remaining_messages(node_id) {
            return;
        }

        let node = &self.nodes[node_id];
        if node.exited_message_requested {
            let timestamp = self.timestamp_source.now();
            let Some(launched_by) = node.launched_by.and_then(|id| self.nodes.get(id)) else {
                return;
            };
            // box the future to because of recursing in an async function
            let future = self.dispatch(
                None,
                timestamp,
                Message::new(
                    &self.core_name,
                    &launched_by.name,
                    None,
                    Exited {
                        name: node.name.clone(),
                        exit_code: node.exit_code().unwrap(),
                    },
                ),
            );
            Box::pin(future).await.unwrap();
        }
        self.cleanup_node(node_id);
    }

    /// split a string into the program and args
    /// for now, it just splits the string using the space character,
    /// taking the first element as the program and the remaining elements as the args
    pub fn split_command(command: impl AsRef<str>) -> ExecutableCommand {
        Self::make_command(command.as_ref().split(" ").map(|s| s.to_string()))
    }

    /// make a command from an iterator of strings. The first element becomes the program,
    /// the remaining elements become the args
    pub fn make_command(command: impl IntoIterator<Item = String>) -> ExecutableCommand {
        let mut command = command.into_iter();
        let program = command.next().unwrap_or_default();
        let args = command.collect::<Vec<_>>();
        ExecutableCommand { program, args }
    }
}
