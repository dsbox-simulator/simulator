use std::future::Future;
use std::pin::{pin, Pin};
use std::task::{Context, Poll};
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
    Lines,
};
use tokio::select;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};

use libproto::Message;

use crate::process::{ProcessCommand, ProcessEvent};

pub async fn process_io_helper<I, O, E, C>(
    event_sender: Sender<ProcessEvent>,
    mut command_receiver: UnboundedReceiver<ProcessCommand>,
    stdin: I,
    stdout: O,
    stderr: E,
    child: C,
) where
    I: AsyncWrite + Unpin,
    O: AsyncRead + Unpin,
    E: AsyncRead + Unpin,
    C: Future<Output = i32>,
{
    let mut stdout_closed = false;
    let mut stderr_closed = false;
    let mut stdin = Some(BufWriter::new(stdin));
    let mut stdout = BufReader::new(stdout).lines();
    let mut stderr = BufReader::new(stderr).lines();
    let mut child = pin!(child);
    loop {
        select! {
            stdout_line = stdout.next_line(), if !stdout_closed => {
                let Ok(Some(line)) = stdout_line else { stdout_closed = true; continue; };
                match Message::from_json(&line) {
                    Ok(message) => event_sender.send(ProcessEvent::Message(message)).await.ok(),
                    Err(error) => event_sender.send(ProcessEvent::SerializeError {
                        raw_message: line.clone(),
                        error: error.to_string(),
                    }).await.ok()
                };
            },
            stderr_line = stderr.next_line(), if !stderr_closed => {
                let Ok(Some(mut log)) = stderr_line else { stderr_closed = true; continue; };
                if let Ok((more, closed)) = poll_more_lines(&mut stderr).await {
                    stderr_closed = closed;
                    log.push('\n');
                    log.push_str(&more);
                } else {
                    stderr_closed = true;
                    continue;
                }
                event_sender.send(ProcessEvent::Log(log.to_owned())).await.ok();
            }
            command = command_receiver.recv() => {
                let Some(command) = command else {
                    // close the stdin handle by taking & dropping it
                    if let Some(mut stdin) = stdin.take() {
                        stdin.flush().await.ok();
                    }
                    continue;
                };
                match command {
                    ProcessCommand::Deliver(message) => {
                        let Some(stdin) = stdin.as_mut() else { continue; };
                        if let Err(error) = write_message(message, stdin).await {
                            log::error!("failed to deliver message to process: {error}");
                        }
                    }
                }
            },
            exit_code = &mut child => {
                event_sender.send(ProcessEvent::Exited(exit_code)).await.ok();
                break;
            },
        }
    }
}

async fn write_message(
    message: Message,
    mut to: impl AsyncWriteExt + Unpin,
) -> std::io::Result<()> {
    to.write_all(message.to_json().as_bytes()).await?;
    to.write_all(b"\n").await?;
    to.flush().await?;
    Ok(())
}

async fn poll_more_lines<R>(lines: &mut Lines<R>) -> tokio::io::Result<(String, bool)>
where
    R: AsyncBufRead + Unpin,
{
    PollMoreLines(lines).await
}

struct PollMoreLines<'a, R>(&'a mut Lines<R>);

impl<'a, R> Future for PollMoreLines<'a, R>
where
    R: AsyncBufRead + Unpin,
{
    type Output = tokio::io::Result<(String, bool)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = &mut self.get_mut().0;
        let mut closed = false;
        let mut extra_lines = String::new();
        while let Poll::Ready(result) = Pin::new(&mut **inner).poll_next_line(cx) {
            if let Some(result) = result? {
                extra_lines.push('\n');
                extra_lines.push_str(&result);
            } else {
                closed = true;
                break;
            }
        }
        Poll::Ready(Ok((extra_lines, closed)))
    }
}
