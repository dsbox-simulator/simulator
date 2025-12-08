use std::task::{Context, Poll};
use crate::process::ProcessCommand;
use crate::process::event::ProcessEventOrExit;
use crate::process::runner::{CommandSender, EventReceiver};
use tokio::task::JoinHandle;

pub struct RunningHandle {
    sender: CommandSender,
    receiver: EventReceiver,
    join_handle: JoinHandle<i32>,
    was_aborted: bool,
    exit_status: ExitStatus,
    commandline: String,
}

enum ExitStatus {
    None,
    Exited(i32),
    Aborted,
}

impl RunningHandle {
    pub fn new(
        sender: CommandSender,
        receiver: EventReceiver,
        handle: JoinHandle<i32>,
        commandline: String,
    ) -> Self {
        Self {
            sender,
            receiver,
            join_handle: handle,
            was_aborted: false,
            exit_status: ExitStatus::None,
            commandline,
        }
    }

    /// Send a [`ProcessCommand`] to the process.
    pub fn send(&self, value: ProcessCommand) -> bool {
        self.sender.send(value).is_ok()
    }

    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<ProcessEventOrExit>> {
        let receiver_closed_and_empty = match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(event)) => return Poll::Ready(Some(ProcessEventOrExit::Event(event))),
            Poll::Ready(None) => true,
            Poll::Pending => false,
        };

        if self.exit_status.is_none() {
            match std::pin::pin!(&mut self.join_handle).poll(cx) {
                Poll::Ready(exit) => {
                    let exit_code = exit.unwrap_or(1);
                    self.exit_status = ExitStatus::Exited(exit_code);
                    return Poll::Ready(Some(ProcessEventOrExit::Exited(exit_code)));
                }
                Poll::Pending => {
                    if self.was_aborted {
                        self.join_handle.abort();
                        self.exit_status = ExitStatus::Aborted;
                        return Poll::Ready(Some(ProcessEventOrExit::Aborted));
                    }
                }
            }
        };

        if receiver_closed_and_empty && !self.exit_status.is_none() {
            // all messages were received and the exit status is set (i.e. the exit event was received): this handle is done
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    /// This drops the `command_sender`, so that threads waiting for [`ProcessCommand`]s from the
    /// [`Core`](crate::core::Core) stop waiting and terminate.
    pub fn begin_shutdown(&self) {
        log::trace!("begin shutdown of process `{}`", self.commandline());
        self.send(ProcessCommand::Shutdown);
    }

    /// This waits for the tasks that handle the processes IO to finish
    pub fn terminate(&mut self) {
        log::trace!("aborting process `{}`", self.commandline());
        self.send(ProcessCommand::Abort);
        self.receiver.close();
        self.was_aborted = true;
    }

    /// Returns `true` if the process has stopped running and all messages have been received
    pub fn has_finished(&self) -> bool {
        if !self.receiver.is_empty() {
            return false;
        }
        !self.exit_status.is_none()
    }

    /// returns the exit code of the process, if it has exited and was not aborted.
    /// The exit code might be `Some` while [`Process::has_finished`] returns false
    /// if the process has exited, but there are still process events to receive.
    /// It is however guaranteed to be `Some` if [`Process::has_finished`] returns true,
    /// by the fact that each process emits a [`ProcessEvent::Exited`] exactly once.
    pub fn exit_code(&self) -> Option<i32> {
        match self.exit_status {
            ExitStatus::None => None,
            ExitStatus::Exited(exit_code) => Some(exit_code),
            ExitStatus::Aborted => None,
        }
    }

    /// returns the full commandline (path + args) that was used to launch the process
    pub fn commandline(&self) -> &str {
        &self.commandline
    }
}

impl ExitStatus {
    fn is_none(&self) -> bool {
        matches!(self, ExitStatus::None)
    }
}
