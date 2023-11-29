//! Commands used to control the execution of the simulation

/// A command for the [`Core`](crate::core::Core) to control its execution
pub enum RemoteCommand {
    /// Pauses the deliver of [`Message`](libproto::Message)s in the [`Core`](crate::core::Core).
    Pause,
    /// Executes a single step. the [`Core`](crate::core::Core) will deliver a single [`Message`](libproto::Message) and then pause again.
    Step,
    /// Resumes execution normally.
    Resume,
}