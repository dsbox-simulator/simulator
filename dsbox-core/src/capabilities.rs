//! Capabilities that define which node can invoke which system messages.
//! Capabilities are registered together with commands, i.e. if a command registered under the
//! name `"test"`, with capabilities `Capabilities::Monitor`, each node spawned from that command
//! inherits `Capabilities::Monitor`.

use enumflags2::BitFlags;

/// Flags for which system message is allowed to be sent to the core, per-node
#[enumflags2::bitflags(default=Break)]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    /// Capability to send a [libproto::system::control::Break] Message the core
    /// in interactive mode.
    Break,

    /// Capability to launch new nodes with a  [`Launch`](libproto::system::Launch) Message
    LaunchNodes,

    /// Capability to register aliases of oneself with an [`Alias`](libproto::system::Alias) Message
    LaunchAlias,

    /// Capability to reset all nodes and aliases that a node has launched/registered
    /// with a [`Reset`](libproto::system::Reset) Message
    Reset,

    /// Capability to monitor messages from/to a specific set of nodes with a
    /// [`BeginMonitor`](libproto::system::BeginMonitor) Message
    Monitor,

    /// Capability to control the execution of the core using the
    /// [`control`](libproto::system::control) Messages
    ControlCore,

    /// Capability to subscribe to all events using a
    /// [`SubscribeEvents`](libproto::system::event::SubscribeEvents) Message
    SubscribeEvents,
}

pub fn has_capability(capabilities: BitFlags<Capability>, message_type: impl AsRef<str>) -> bool {
        use libproto::system::*;
        use libproto::system::control::*;
        use libproto::Payload;
    use libproto::system::event::SubscribeEvents;
    match message_type.as_ref() {
                Break::TYPE => capabilities.contains(Capability::Break),
                Launch::TYPE => capabilities.contains(Capability::LaunchNodes),
                Alias::TYPE => capabilities.contains(Capability::LaunchAlias),
                Reset::TYPE => capabilities.contains(Capability::Reset),
                BeginMonitor::TYPE => capabilities.contains(Capability::Monitor),
                Control::TYPE => capabilities.contains(Capability::ControlCore),
                SubscribeEvents::TYPE => capabilities.contains(Capability::SubscribeEvents),
                _ => false
        }
}

