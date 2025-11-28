use crate::process::runner::lines_helper::LinesHelper;
use crate::process::runner::{CommandReceiver, EventSender};
use crate::process::{ProcessCommand, ProcessEvent};
use libproto::Message;
use std::io;
use std::ops::Add;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::time::Instant;

pub trait ChildHandle {
    fn stdin(&mut self) -> Option<impl AsyncWrite + Unpin + 'static>;
    fn stdout(&mut self) -> Option<impl AsyncRead + Unpin + 'static>;
    fn stderr(&mut self) -> Option<impl AsyncRead + Unpin + 'static>;

    fn abort(&mut self);
    fn wait(&mut self) -> impl Future<Output = i32>;
}

pub async fn io_helper(
    sender: EventSender,
    mut receiver: CommandReceiver,
    mut child: impl ChildHandle,
) -> i32 {
    let mut stdout = LinesHelper::new(child.stdout().unwrap());
    let mut stderr = LinesHelper::new(child.stderr().unwrap());
    let mut stdin = child.stdin().unwrap();
    let mut exit_code = None;

    while !stdout.is_closed() || !stderr.is_closed() || !receiver.is_closed() || exit_code.is_none()
    {
        tokio::select! {
            stdout_line = stdout.line(), if !stdout.is_closed() => {
                handle_message(stdout_line, &sender).await
            },
            stderr_line = stderr.line(), if !stderr.is_closed() => {
                handle_log(stderr_line, &sender, &mut stderr).await
            },
            command = receiver.recv(), if !receiver.is_closed() => {
                handle_command(command, &mut stdin, &mut child).await;
            }
            code = child.wait(), if exit_code.is_none() => {
                exit_code = Some(code);
            }
        }
    }
    exit_code.expect("expected to have an exit code")
}

async fn handle_message(line: io::Result<Option<String>>, sender: &EventSender) {
    let Ok(Some(line)) = line else {
        return;
    };
    let event = match Message::from_json(&line) {
        Ok(message) => ProcessEvent::Message(message),
        Err(serialize_error) => ProcessEvent::SerializeError {
            raw_message: line,
            error: serialize_error.to_string(),
        },
    };
    sender.send(event).await.ok();
}

async fn handle_log<T: AsyncRead + Unpin>(
    line: io::Result<Option<String>>,
    sender: &EventSender,
    stderr: &mut LinesHelper<T>,
) {
    let Ok(Some(mut line)) = line else {
        return;
    };
    // poll for more log lines for 10 milliseconds
    let deadline = Instant::now().add(Duration::from_millis(10));
    while let Ok(next_line) = tokio::time::timeout_at(deadline, stderr.line()).await {
        let Ok(Some(next_line)) = next_line else {
            break;
        };
        line.push('\n');
        line.push_str(&next_line);
    }
    sender.send(ProcessEvent::Log(line)).await.ok();
}

async fn handle_command(
    command: Option<ProcessCommand>,
    mut stdin: impl AsyncWrite + Unpin,
    child: &mut impl ChildHandle,
) {
    let Some(command) = command else {
        return;
    };
    match command {
        ProcessCommand::Deliver(message) => {
            let mut message = message.to_string();
            message.push('\n');
            stdin.write_all(message.as_bytes()).await.ok();
            stdin.flush().await.ok();
        }
        ProcessCommand::Abort => {
            child.abort();
        }
    }
}
