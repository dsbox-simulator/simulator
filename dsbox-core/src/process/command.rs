use libproto::Message;

/// Command that the core can send to a process. Only for delivering messages (for now)
pub enum ProcessCommand {
    /// Deliver the [`Message`] to the process (i.e write it to its `stdin`).
    Deliver(Message),
}