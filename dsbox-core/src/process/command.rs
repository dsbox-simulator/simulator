use libproto::Message;

/// Command that the core can send to a process. Only for delivering messages (for now)
pub enum ProcessCommand {
    /// Deliver the [`Message`] to the process (i.e write it to its `stdin`).
    Deliver(Message),
    /// Shut down the process gracefully (i.e. by closing standard input handles or similar)
    Shutdown,
    /// Abort the process, by any means necessary (i.e. a native process via SIGKILL, or similar)
    Abort,
}