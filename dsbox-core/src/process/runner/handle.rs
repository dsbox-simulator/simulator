use crate::command::ExecutableCommand;
use crate::process::event::ProcessEventOrExit;
use crate::process::runner::{CommandSender, EventReceiver};
use crate::process::{Process, ProcessCommand, ProcessEvent};
use tokio::task::JoinHandle;

pub struct RunningHandle {
    sender: Option<CommandSender>,
    receiver: EventReceiver,
    join_handle: JoinHandle<i32>,
    exit_code: Option<i32>,
    command: ExecutableCommand,
}

impl RunningHandle {
    pub fn new(
        sender: CommandSender,
        receiver: EventReceiver,
        handle: JoinHandle<i32>,
        command: ExecutableCommand,
    ) -> Self {
        Self {
            sender: Some(sender),
            receiver,
            join_handle: handle,
            exit_code: None,
            command,
        }
    }

    /// Send a [`ProcessCommand`] to the process.
    pub fn send(&self, value: ProcessCommand) -> bool {
        if let Some(sender) = &self.sender {
            sender.send(value).is_ok()
        } else {
            false
        }
    }

    pub async fn recv(&mut self) -> Option<ProcessEventOrExit> {
        tokio::select! {
            exit_code = &mut self.join_handle, if self.exit_code.is_none() => {
                let exit_code = exit_code.unwrap_or(1);
                self.exit_code = Some(exit_code);
                Some(ProcessEventOrExit::Exited(exit_code))
            }
            event = self.receiver.recv() => {
                Some(ProcessEventOrExit::Event(event?))
            }
        }
    }

    /// This drops the `command_sender`, so that threads waiting for [`ProcessCommand`]s from the
    /// [`Core`](crate::core::Core) stop waiting and terminate.
    pub fn begin_shutdown(&mut self) {
        if self.sender.is_some() {
            log::trace!("begin shutdown of process `{}`", self.commandline());
        }
        self.sender.take();
    }

    /// This waits for the tasks that handle the processes IO to finish
    pub async fn terminate(&mut self) {
        self.begin_shutdown();
        self.receiver.close();
        self.exit_code = Some((&mut self.join_handle).await.unwrap_or(1));
    }

    /// Returns `true` if the process has stopped running and all messages have been received
    pub fn has_finished(&self) -> bool {
        if !self.receiver.is_empty() {
            return false;
        }
        self.join_handle.is_finished()
    }
    /// returns the exit code of the process, if it has exited.
    /// The exit code might be `Some` while [`Process::has_finished`] returns false
    /// if the process has exited, but there are still process events to receive.
    /// It is however guaranteed to be `Some` if [`Process::has_finished`] returns true,
    /// by the fact that each process emits a [`ProcessEvent::Exited`] exactly once.
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    /// returns the full commandline (path + args) that was used to launch the process
    pub fn commandline(&self) -> String {
        self.command.to_string()
    }
}
