use crate::process::runner::io_helper::ChildHandle;
use crate::process::runner::{io_helper, CommandReceiver, EventSender, Runner};
use crate::process::ProcessEvent;
use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::process::{Child, Command};

pub struct NativeRunner;

impl Runner for NativeRunner {
    fn run(
        &mut self,
        args: Vec<String>,
        sender: EventSender,
        receiver: CommandReceiver,
    ) -> impl Future<Output = i32> + 'static {
        run_native(args, sender, receiver)
    }
}

async fn run_native(
    args: Vec<String>,
    sender: EventSender,
    receiver: CommandReceiver,
) -> i32 {
    let child = match Command::new(&args[0])
        .args(&args[1..])
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

    io_helper::io_helper(sender, receiver, child).await
}

impl ChildHandle for Child {
    fn stdin(&mut self) -> Option<impl AsyncWrite + Unpin + 'static> {
        self.stdin.take()
    }

    fn stdout(&mut self) -> Option<impl AsyncRead + Unpin + 'static> {
        self.stdout.take()
    }

    fn stderr(&mut self) -> Option<impl AsyncRead + Unpin + 'static> {
        self.stderr.take()
    }

    fn abort(&mut self) {
        self.start_kill().ok();
    }

    fn wait(&mut self) -> impl Future<Output = i32> {
        async move {
            self.wait()
                .await
                .map(|s| s.code())
                .ok()
                .flatten()
                .unwrap_or(1)
        }
    }
}
