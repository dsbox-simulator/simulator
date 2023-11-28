//! Commands used to control the execution of the simulation

/// A command for the [`Core`](crate::core::Core) to control its execution
pub enum RemoteCommand {
    /// Pauses execution. What this exactly means is still to be determined.
    Pause,
    /// Executes a single step. What this exactly means is still to be determined.
    Step,
    /// Resumes execution.
    Resume,
}