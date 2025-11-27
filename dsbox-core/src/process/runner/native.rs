use crate::command::ExecutableCommand;
use crate::process::runner::{io_helper, CommandReceiver, Runner, EventSender};
use crate::process::ProcessEvent;
use std::process::Stdio;
use tokio::process::Command;
pub struct NativeRunner;

impl Runner for NativeRunner {
    fn run(
        &mut self,
        command: ExecutableCommand,
        sender: EventSender,
        receiver: CommandReceiver,
    ) -> impl Future<Output = i32> + 'static {
        run_native(command, sender, receiver)
    }
}

async fn run_native(command: ExecutableCommand, sender: EventSender, receiver: CommandReceiver) -> i32 {
    let mut child = match Command::new(command.program)
        .args(command.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            sender.send(ProcessEvent::Log(e.to_string())).await.ok();
            return -1;
        }
    };

    io_helper::io_helper(
        sender,
        receiver,
        child.stdin.take().unwrap(),
        child.stdout.take().unwrap(),
        child.stderr.take().unwrap(),
        async move {
            child
                .wait()
                .await
                .map(|s| s.code())
                .ok()
                .flatten()
                .unwrap_or(1)
        },
    )
    .await
}