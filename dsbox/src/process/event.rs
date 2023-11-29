//! A [`ProcessEvent`] encompasses everything that happens in a process.
//! This includes sending [`Message`]s, logging lines, exiting, and errors,
//! and might be expanded in the future.
use libproto::Message;

/// An event from a process
pub struct ProcessEvent {
    /// The unique id of the process in the [`Core`](crate::core::Core). This is necessary,
    /// because all [`ProcessEvent`]s from all processes are sent over a single channel.
    pub source_id: usize,
    /// The kind of event that happened.
    pub kind: ProcessEventKind,
}

/// Describes what happened in the process
pub enum ProcessEventKind {
    /// A [`Message`] was written to the processes `stdout`.
    Message(Message),
    /// A log line was written to the processes `stderr`.
    Log(String),
    /// The process exited
    Exited(i32),
    /// Something was written to the processes `stdout` that could not be deserialized into a [`Message`]
    SerializeError(String, serde_json::Error),
}

impl ProcessEvent {
    pub fn new(source_id: usize, kind: ProcessEventKind) -> Self {
        Self {
            source_id,
            kind,
        }
    }
}