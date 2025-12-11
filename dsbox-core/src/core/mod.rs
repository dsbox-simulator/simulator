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

use crate::capabilities::Capability;
use crate::command::RunnerCommand;
use crate::core::error::{CoreError, DispatchErrorKind};
use crate::core::monitor::MonitorSession;
use crate::core::node::NodeId;
use crate::core::node_manager::NodeManager;
use crate::core::timer_manager::{Timer, TimerKind, TimerManager};
use crate::log_color;
use crate::log_color::log_marker_ansi_color;
use crate::process::RunnerManger;
use crate::process::RunningHandle;
use crate::process::{ProcessCommand, ProcessEvent, ProcessEventOrExit};
pub use builder::Builder;
use libproto::system::control::{Break, Control};
use libproto::system::event::{Event, PublishEvent, SubscribeEvents, Timestamp};
use network::Network;
use timestamp_source::TimestampSource;

mod builder;
pub mod error;
mod monitor;
mod network;
mod node;
mod node_manager;
mod timer_manager;
mod timestamp_source;
mod version;

/// The core of the simulation
///
/// This struct contains all state of the simulation and is used to drive execution forwards
/// by collecting [`ProcessEvent`]s from processes, delivering [`Message`]s and listening for
/// remote control commands.
pub struct Core {
    /// the name of the simulation core
    /// this name must be used as the source/destination for "core" messages and is used in core logs
    core_name: String,
    /// list of commands and node names that should be launched initially
    launch_initial: Vec<InitialLaunch>,
    /// Manages all nodes that are participating in the simulation
    nodes: NodeManager,
    /// registry of commands, that can be launched through [`Launch`] Messages, and their capabilities
    commands: HashMap<String, (RunnerCommand, BitFlags<Capability>)>,
    /// runs new nodes
    runner_manager: RunnerManger,
    /// `true` if the program was started in interactive mode (i.e. with the user interface enabled)
    interactive: bool,
    /// The current execution state (i.e. running/stepping/paused...)
    state: CoreState,
    /// source for logical timestamps within a single run.
    /// Is automatically reset after a `reset` command is received
    timestamp_source: TimestampSource,
    /// list of all active [`MonitorSession`]s
    monitor_sessions: Vec<MonitorSession>,
    /// list of all nodes that have subscribed to events
    event_subscribers: Vec<NodeId>,
    /// the [`Network`] contains all [`Message`]s that are sent, but not yet delivered
    network: Network,
    /// a manager for outstanding timers
    timer_manager: TimerManager,
}

/// describes a node that should be launched initially, on [`Core::run`]
struct InitialLaunch {
    pub command: String,
    pub name: String,
    pub requires_registration: bool,
    pub weak: bool,
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
        Self {
            core_name: builder.core_name,
            commands: builder.commands,
            runner_manager: builder.runner_manager,
            launch_initial: builder.launch_initial,
            nodes: NodeManager::new(),
            interactive: builder.interactive,
            timestamp_source: TimestampSource::new(),
            state: if builder.interactive {
                CoreState::Paused
            } else {
                CoreState::Running
            },
            event_subscribers: Vec::new(),
            monitor_sessions: Vec::new(),
            network: Network::new(),
            timer_manager: TimerManager::new(),
        }
    }
}

impl Core {
    /// returns a new [`Builder`] for configuring and building a new [`Core`]
    pub fn builder() -> Builder {
        Builder::new()
    }

    async fn setup(&mut self) -> Result<(), CoreError> {
        // publish an initial "reset" event, so that the webapp can reset its state when "dsbox"
        // is re-started
        let timestamp = self.timestamp_source.now();
        self.publish_event(Event::reset(timestamp));

        for init in std::mem::take(&mut self.launch_initial) {
            let node = self
                .launch(
                    &init.command,
                    None,
                    init.requires_registration,
                    false,
                    init.weak,
                    init.name.clone(),
                )
                .await?;
            let commandline = node.commandline().to_owned();
            let timestamp = self.timestamp_source.now();
            self.dispatch(
                None,
                timestamp,
                Message::new(
                    &self.core_name,
                    &init.name,
                    None,
                    Init {
                        name: init.name.clone(),
                        core_name: self.core_name.clone(),
                        core_version: version::current(),
                    },
                ),
            )
            .await?;
            let timestamp = self.timestamp_source.now();
            self.publish_event(Event::node_launched(timestamp, init.name, commandline))
        }
        Ok(())
    }

    /// starts the execution. This function consumes the passed [`Core`] because it cannot be restarted
    /// after [`Core::run`] returns.
    pub async fn run(mut self) {
        // launch test node/publish initial reset event
        if let Err(e) = self.setup().await {
            self.log_core_error(e).await;
            return;
        }
        loop {
            let num_running = self.nodes.len();
            let strong_running = self.nodes.iter().filter(|n| !n.weak).count();

            if strong_running == 0 && self.network.is_empty() {
                // shut down all remaining (weak) nodes when all strong nodes have shut down
                // and all messages have been delivered
                self.begin_shutdown_timeout(|_| true, Duration::from_secs(1));
            }

            if num_running == 0 {
                // all nodes have shut down or were terminated: finish running
                break;
            }

            if let Err(e) = self.step(num_running).await {
                self.log_core_error(e).await;
            }
        }
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
                process_event_or_exit = self.nodes.recv_any(), if num_running > 0 => {
                    if let Some(first) = process_event_or_exit {
                        let mut events = vec![first];
                        // try to receive at most 16 more messages that arrive at most 1 millisecond apart
                        self.nodes.try_recv_more(&mut events, 16, Duration::from_millis(1)).await;
                        for (event, node_id) in events {
                            let ts = self.timestamp_source.now();
                            self.handle_process_event_or_exit(ts, node_id, event).await?;
                        }
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
    async fn handle_process_event_or_exit(
        &mut self,
        timestamp: Timestamp,
        source_id: NodeId,
        process_event_or_exit: ProcessEventOrExit,
    ) -> Result<bool, CoreError> {
        match process_event_or_exit {
            ProcessEventOrExit::Event(process_event) => {
                self.handle_process_event(timestamp, source_id, process_event)
                    .await
            }
            ProcessEventOrExit::Exited(exit_code) => {
                self.process_exited(timestamp, source_id, Some(exit_code))
                    .await?;
                Ok(true)
            }
            ProcessEventOrExit::Aborted => {
                self.process_exited(timestamp, source_id, None).await?;
                Ok(true)
            }
        }
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
            ProcessEvent::SerializeError { raw_message, error } => Err(CoreError::SerializeError {
                source: self.nodes[source_id].commandline().to_owned(),
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

        self.publish_event(Event::send_message(timestamp, message.clone()));

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
            self.publish_event(Event::deliver_message(timestamp, sent_timestamp.logical));
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
            TimerKind::ShutdownTimeout { node_ids } => {
                for node_id in node_ids {
                    let node = &mut self.nodes[node_id];
                    if !node.has_finished() {
                        node.terminate();
                    }
                }
                Ok(())
            }
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
                self.initiate_reset(source_id);
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
                self.handle_control_message(message.payload::<Control>().unwrap())
                    .await
            }
            SubscribeEvents::TYPE => {
                assert_has_capability!(SubscribeEvents);
                if let Some(source) = source {
                    self.subscribe_events(source);
                }
                Ok(())
            }
            ty => Err(CoreError::UnknownCoreMessage {
                name: message.src,
                ty: ty.to_owned(),
            }),
        }
    }

    fn subscribe_events(&mut self, source: NodeId) {
        self.event_subscribers.push(source);
        // send the new subscriber a `node_launched` message for each node currently running,
        // so that it does not miss any nodes that were launched before subscribing
        let subscriber = &self.nodes[source];
        for node in &self.nodes {
            if node.id == subscriber.id {
                continue;
            }
            let name = node.name.clone();
            let commandline = node.commandline().to_owned();
            let timestamp = self.timestamp_source.now();
            let event = Event::node_launched(timestamp, name, commandline);
            subscriber.send(ProcessCommand::Deliver(Message::new(
                &self.core_name,
                &subscriber.name,
                None,
                PublishEvent { event },
            )));
        }
    }

    fn initiate_reset(&mut self, source_id: NodeId) {
        let removed_aliases = self.nodes.remove_aliases_of(source_id);
        self.cleanup_aliases(removed_aliases);

        let num_shutdown = self.begin_shutdown_timeout(
            |node| node.launched_by == Some(source_id),
            Duration::from_secs(1),
        );
        self.nodes[source_id].reset_requested = true;
        if num_shutdown == 0 {
            self.finish_reset(source_id);
        }
    }

    fn check_finish_reset(&mut self, source_id: NodeId) {
        let Some(node) = self.nodes.get(source_id) else {
            return;
        };
        if !node.reset_requested {
            return;
        }
        if self
            .nodes
            .iter()
            .any(|node| node.launched_by == Some(node.id))
        {
            return;
        }
        self.finish_reset(source_id);
    }

    fn finish_reset(&mut self, source_id: NodeId) {
        let node = &mut self.nodes[source_id];
        node.reset_requested = false;
        node.send(ProcessCommand::Deliver(Message::new(
            &self.core_name,
            &node.name,
            None,
            ResetFinished {},
        )));
    }

    async fn handle_control_message(&mut self, control_message: Control) -> Result<(), CoreError> {
        match control_message {
            Control::Break => self.state = CoreState::Paused,
            Control::Step => self.state = CoreState::Stepping,
            Control::Resume => self.state = CoreState::Running,
            Control::Deliver { sent_timestamp } => {
                self.deliver_by_timestamp(sent_timestamp).await?
            }
            Control::Drop { sent_timestamp } => self.drop_by_timestamp(sent_timestamp).await,
            Control::Shutdown => {
                self.begin_shutdown_timeout(|_| true, Duration::from_secs(1));
            }
        }
        Ok(())
    }

    async fn drop_by_timestamp(&mut self, sent_timestamp: usize) {
        self.network.remove_one(sent_timestamp);
        let timestamp = self.timestamp_source.now();
        self.publish_event(Event::drop_message(timestamp, sent_timestamp));
    }

    async fn deliver_by_timestamp(&mut self, sent_timestamp: usize) -> Result<(), CoreError> {
        if let Some((timestamp, source_id, message)) = self.network.remove_one(sent_timestamp) {
            self.deliver(timestamp, source_id, message).await?
        }
        Ok(())
    }

    fn begin_shutdown_timeout<P>(&mut self, predicate: P, timeout: Duration) -> usize
    where
        P: FnMut(&&Node) -> bool,
    {
        let node_ids = self
            .nodes
            .iter()
            .filter(predicate)
            .map(|node| {
                node.begin_shutdown();
                node.id
            })
            .collect::<Vec<_>>();
        let num_shutdown = node_ids.len();
        self.timer_manager.add(
            Instant::now().add(timeout),
            TimerKind::ShutdownTimeout { node_ids },
        );
        num_shutdown
    }

    fn cleanup_node(&mut self, cleanup_id: NodeId) -> Node {
        let node = &mut self.nodes[cleanup_id];
        if !node.has_finished() {
            node.terminate();
        }

        self.cleanup_aliases(self.nodes.aliases_of(cleanup_id));

        // remove the node and all its aliases
        let node = self.nodes.remove(cleanup_id);

        // remove all active timers for that node
        self.timer_manager.retain(|timer| match &timer.kind {
            TimerKind::TimerService { .. } => true,
            TimerKind::ExpectRegistry { node_id } => *node_id != cleanup_id,
            &TimerKind::ShutdownTimeout { .. } => true,
        });
        node.unwrap()
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
            &TimerKind::ShutdownTimeout { .. } => true,
        })
    }

    /// creates an alias for a server
    async fn create_alias(&mut self, for_id: NodeId, alias: Alias) {
        let error = match self.nodes.add_alias(for_id, alias.name.clone()) {
            Ok(Some(node)) => {
                let timestamp = self.timestamp_source.now();
                let commandline = node.commandline().to_owned();
                self.publish_event(Event::node_launched(timestamp, alias.name, commandline));
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
                let commandline = node.commandline().to_owned();
                let timestamp = self.timestamp_source.now();
                self.publish_event(Event::node_launched(timestamp, name, commandline));
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
    /// TODO: clean this up an merge all of the launch, launch_single, etc. into one?
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
                false,
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
                },
            ),
        )
        .await?;
        Ok(&mut self.nodes[node_id])
    }

    /// launches a new process and creates the corresponding node
    async fn launch(
        &mut self,
        command_name: &str,
        launched_by: Option<NodeId>,
        requires_registration: bool,
        exited_message_requested: bool,
        weak: bool,
        name: String,
    ) -> Result<&mut Node, CoreError> {
        let (handle, capabilities) = self.run_process(command_name).await?;
        let node = self
            .nodes
            .add(Node::new(
                name.clone(),
                launched_by,
                capabilities,
                exited_message_requested,
                requires_registration,
                weak,
                handle,
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

    async fn run_process(
        &mut self,
        command_name: &str,
    ) -> Result<(RunningHandle, BitFlags<Capability>), CoreError> {
        let (command, capabilities) =
            self.commands
                .get(command_name)
                .ok_or_else(|| CoreError::UnknownCommand {
                    command_name: command_name.to_string(),
                    available_commands: self.commands.keys().cloned().collect(),
                })?;
        let handle = self
            .runner_manager
            .run(command)
            .map_err(|_| CoreError::UnknownRunner {
                runner_name: command.runner().to_owned(),
                available_runners: self.runner_manager.available_runners().cloned().collect(),
            })?;
        Ok((handle, *capabilities))
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
        let name = node.name.clone();
        self.publish_event(Event::log(timestamp, name, message));
    }

    async fn log_core_error(&mut self, error: CoreError) {
        let message = format!("simulation core error:\n{error}");
        log::error!("{message}");
        let timestamp = self.timestamp_source.now();
        self.publish_event(Event::log(
            timestamp,
            self.core_name.clone(),
            LogMessage {
                text: message,
                marker: Some(LogMarker {
                    label: "ERR".to_string(),
                    color: Some(LogMarkerColor::Red),
                }),
            },
        ));
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
        exit_code: Option<i32>,
    ) -> Result<(), CoreError> {
        // shutdown all nodes that were launched by this node
        self.begin_shutdown_timeout(
            |node| node.launched_by == Some(source_id),
            Duration::from_secs(1),
        );

        let node = &self.nodes[source_id];
        let name = node.name.clone();
        self.publish_event(Event::node_disconnected(timestamp, name, exit_code));
        if let Some(exit_code) = exit_code {
            log::info!(
                "[{}] command `{}` exited with code {exit_code}",
                node.name,
                node.commandline()
            );
        } else {
            log::info!(
                "[{}] command `{}` was aborted and did not terminate normally",
                node.name,
                node.commandline()
            );
        }
        self.check_exited(source_id).await;
        Ok(())
    }

    fn publish_event(&self, event: Event) {
        for node in self.event_subscribers.iter().copied() {
            // monitor events are not dispatched via the network. Instead, they are delivered directly
            // to the target node. Among other reasons, this de-clutters the message log (monitor events
            // should not be the target of any kind of debugging/visualization)
            let node = &self.nodes[node];
            node.send(ProcessCommand::Deliver(Message::new(
                &self.core_name,
                &node.name,
                None,
                PublishEvent {
                    event: event.clone(),
                },
            )));
        }
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
        if node.exited_message_requested
            && let Some(launched_by) = node.launched_by.and_then(|id| self.nodes.get(id))
        {
            let timestamp = self.timestamp_source.now();
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
        let node = self.cleanup_node(node_id);
        if let Some(launched_by) = node.launched_by {
            self.check_finish_reset(launched_by);
        }
    }
}
