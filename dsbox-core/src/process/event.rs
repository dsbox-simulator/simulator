//! A [`ProcessEvent`] encompasses everything that happens in a process.
//! This includes sending [`Message`]s, logging lines, exiting, and errors,
//! and might be expanded in the future.

use libproto::Message;

/// Describes what happened in the process
#[derive(Debug)]
pub enum ProcessEvent {
    /// A [`Message`] was written to the processes `stdout`.
    Message(Message),
    /// A log line was written to the processes `stderr`.
    Log(String),
    /// Something was written to the processes `stdout` that could not be deserialized into a [`Message`]
    SerializeError { raw_message: String, error: String },
}

/// Describes what happened in the process or that the process has exited.
/// After a [`ProcessEventOrExit::Exited`], more process events may arrive
/// if they were still queued up
///
/// The reason this is split into a separate enum is that a process runner
/// may send [`ProcessEvent`]s to the core, but can only signal the exit (with an exit code)
/// by returning from the [`crate::process::runner::Runner::run`] implementation
#[derive(Debug)]
pub(crate) enum ProcessEventOrExit {
    /// an event happened (a message, log message, etc.)
    Event(ProcessEvent),
    /// The process exited
    Exited(i32),
    /// The process was aborted by the core
    Aborted,
}