use std::future::Future;
use std::pin::pin;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::select;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::sync::oneshot;

use libproto::Message;

use crate::process::{ProcessCommand, ProcessEvent};

pub async fn process_io_helper<I, O, E, C>(event_sender: Sender<ProcessEvent>, mut command_receiver: UnboundedReceiver<ProcessCommand>, stdin: I, stdout: O, stderr: E, mut child: C, finished: oneshot::Sender<()>)
    where I: AsyncWrite + Unpin,
          O: AsyncRead + Unpin,
          E: AsyncRead + Unpin,
          C: Future<Output=i32> {
    let mut stdout_closed = false;
    let mut stderr_closed = false;
    let mut finished = Some(finished);
    let mut stdin = Some(BufWriter::new(stdin));
    let mut stdout = BufReader::new(stdout).lines();
    let mut stderr = BufReader::new(stderr).lines();
    let mut child = pin!(child);
    while finished.is_some() || !stdout_closed || !stderr_closed || stdin.is_some() {
        select! {
            stdout_line = stdout.next_line(), if !stdout_closed => {
                let Ok(Some(line)) = stdout_line else { stdout_closed = true; continue; };
                let send_result = match Message::from_json(&line) {
                    Ok(message) => event_sender.send(ProcessEvent::Message(message)).await,
                    Err(error) => event_sender.send(ProcessEvent::SerializeError {
                        raw_message: line.clone(),
                        error: error.to_string(),
                    }).await
                };
                if send_result.is_err() { break; }
            },
            stderr_line = stderr.next_line(), if !stderr_closed => {
                let Ok(Some(line)) = stderr_line else { stderr_closed = true; continue; };
                if event_sender.send(ProcessEvent::Log(line.to_owned())).await.is_err() {
                    break;
                }
            }
            command = command_receiver.recv(), if stdin.is_some() => {
                let Some(command) = command else {
                    stdin.take();
                    continue;
                };
                let Some(stdin) = stdin.as_mut() else { continue; };
                match command {
                    ProcessCommand::Deliver(message) => {
                        if stdin.write_all(message.to_json().as_bytes()).await.is_err() {
                            todo!("react to error appropriately")
                        }
                        if stdin.write_all(b"\n").await.is_err() {
                            todo!("react to error appropriately")
                        }
                        if stdin.flush().await.is_err() {
                            todo!("react to error appropriately")
                        }
                    }
                }
            },
            exit_code = &mut child, if finished.is_some() => {
                event_sender.send(ProcessEvent::Exited(exit_code)).await.ok();
                finished.take().unwrap().send(()).ok();
            },
        }
    }
}