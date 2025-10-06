//! A [`ProcessEvent`] encompasses everything that happens in a process.
//! This includes sending [`Message`]s, logging lines, exiting, and errors,
//! and might be expanded in the future.
use libproto::Message;

/// Describes what happened in the process
pub enum ProcessEvent {
    /// A [`Message`] was written to the processes `stdout`.
    Message(Message),
    /// A log line was written to the processes `stderr`.
    Log(String),
    /// The process exited
    Exited(i32),
    /// Something was written to the processes `stdout` that could not be deserialized into a [`Message`]
    SerializeError { raw_message: String, error: String },
}