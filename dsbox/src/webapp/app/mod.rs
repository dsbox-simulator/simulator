mod poll_optional;
mod rpc_util;

use crate::cli::Args;
use crate::webapp::app::poll_optional::poll_optional_join_handle;
use crate::webapp::app::rpc_util::RpcFn;
use crate::webapp::json_rpc::request::Request;
use crate::webapp::json_rpc::response;
use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use dsbox_core::core::error::CoreError;
use dsbox_core::core::event::Event;
use dsbox_core::core::remote_control::RemoteCommand;
use dsbox_core::core::{Builder, Core};

use async_channel::{Receiver, Sender};
use dsbox_core::Command;
use serde::Serialize;
use serde_json::Value;
use std::future::poll_fn;
use std::time::Duration;
use tokio::task::JoinHandle;

pub struct App {
    remote: Sender<RemoteCommand>,
    subscriber: Receiver<Event>,
    core_handle: Option<JoinHandle<()>>,
    commands: Commands,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Commands {
    test_command: Command,
    server_command: Command,
}

impl App {
    pub async fn new(args: Args) -> Result<Self, CoreError> {
        let test_command = Core::split_command(&args.test_command);
        let server_command = Core::make_command(args.server_command.iter().cloned());
        let core = Core::builder(test_command.clone(), server_command.clone())
            .interactive(true)
            .allow_lua_unsafe(args.lua_unsafe)
            .build();

        let subscriber = core.subscribe_events();
        let remote = core.remote_control();
        let core_handle = tokio::task::spawn(async move { core.run().await });
        Ok(Self {
            remote,
            subscriber,
            core_handle: Some(core_handle),
            commands: Commands {
                test_command,
                server_command,
            },
        })
    }

    async fn handle_event(
        &mut self,
        event: Event,
        socket: &mut WebSocket,
    ) -> Result<(), axum::Error> {
        let request = Request::notification("event".to_string(), event)
            .expect("failed to serialize jsonrpc request");
        socket
            .send(Message::Text(Utf8Bytes::from(
                serde_json::to_string(&request).expect("failed to serialize jsonrpc request"),
            )))
            .await
    }

    async fn handle_socket_message(
        &mut self,
        message: Option<Result<Message, axum::Error>>,
        socket: &mut WebSocket,
    ) -> bool {
        match message {
            Some(Ok(Message::Close(_))) => false,
            Some(Ok(Message::Text(message))) => {
                let mut response = Vec::new();
                if let Err(e) = self.dispatch_raw(message.as_bytes(), &mut response).await {
                    log::warn!("Error dispatching jsonrpc request: {e}");
                    false
                } else {
                    if !response.is_empty() {
                        let response_string = String::from_utf8(response)
                            .expect("expected rpc response to be a string");
                        socket
                            .send(Message::Text(Utf8Bytes::from(response_string)))
                            .await
                            .expect("error sending response to websocket");
                    }
                    true
                }
            }
            Some(Ok(_)) => panic!("unknown message type"),
            Some(Err(e)) => {
                log::warn!("websocket error: {e}");
                false
            }
            None => true,
        }
    }

    fn rpc_dispatch_map(method: &str) -> Option<RpcFn> {
        macro_rules! dispatch {
            ($method:ident($($a:ident: $t:ty),*$(,)?)) => {
                Some(Box::new(|app: &mut Self, args: Value| {
                    #[derive(serde::Deserialize)]
                    struct Args {
                        $($a: $t),*
                    }
                    Box::pin(app.dispatch_with_args(args, |app:&mut Self, _args:Args| {
                        app.$method($(_args.$a),*)
                    }))
                }))
            };
        }
        match method {
            "restart" => {
                dispatch!(restart(test_command: Option<Command>, server_command: Option<Command>))
            }
            "break" => dispatch!(break_()),
            "resume" => dispatch!(resume()),
            "step" => dispatch!(step()),
            "current_commands" => dispatch!(current_commands()),
            "deliver" => dispatch!(deliver(sent_timestamp: usize)),
            "drop" => dispatch!(drop(sent_timestamp: usize)),
            "store" => dispatch!(store(key: String, value: Value)),
            "load" => dispatch!(load(key: String)),
            "remove" => dispatch!(remove(key: String)),
            _ => None,
        }
    }

    pub async fn run(mut self, mut socket: WebSocket) {
        loop {
            tokio::select! {
                event = self.subscriber.recv() => {
                    let Ok(event) = event else { break; };
                    if let Err(e) = self.handle_event(event, &mut socket).await {
                        log::warn!("websocket error when sending event message: {e}");
                        break;
                    }
                }
                result = poll_fn(poll_optional_join_handle(&mut self.core_handle)) => {
                    match result {
                        Ok(()) => log::info!("core shut down"),
                        Err(join_error) => log::error!("core panicked: {join_error}"),
                    }
                    break;
                }
                message = socket.recv() => if !self.handle_socket_message(message, &mut socket).await {
                    break;
                },
            }
        }
        self.remote.send(RemoteCommand::Shutdown).await.ok();
        tokio::time::timeout(Duration::from_secs(1), self.core_handle.unwrap())
            .await
            .ok();
    }

    async fn restart(
        &mut self,
        test_command: Option<Command>,
        server_command: Option<Command>,
    ) -> Result<(), response::Error> {
        if let Some(test_command) = &test_command {
            self.commands.test_command = test_command.clone();
        }
        if let Some(server_command) = &server_command {
            self.commands.server_command = server_command.clone();
        }
        self.remote
            .send(RemoteCommand::Restart {
                test_command,
                server_command,
            })
            .await
            .ok();
        Ok(())
    }

    async fn break_(&mut self) -> Result<(), response::Error> {
        self.remote.send(RemoteCommand::Break).await.ok();
        Ok(())
    }

    async fn resume(&mut self) -> Result<(), response::Error> {
        self.remote.send(RemoteCommand::Resume).await.ok();
        Ok(())
    }

    async fn step(&mut self) -> Result<(), response::Error> {
        self.remote.send(RemoteCommand::Step).await.ok();
        Ok(())
    }

    async fn current_commands(&mut self) -> Result<Commands, response::Error> {
        Ok(self.commands.clone())
    }

    async fn deliver(&mut self, sent_timestamp: usize) -> Result<(), response::Error> {
        self.remote
            .send(RemoteCommand::Deliver(sent_timestamp))
            .await
            .ok();
        Ok(())
    }

    async fn drop(&mut self, sent_timestamp: usize) -> Result<(), response::Error> {
        self.remote
            .send(RemoteCommand::Drop(sent_timestamp))
            .await
            .ok();
        Ok(())
    }

    async fn store(&mut self, key: String, value: Value) -> Result<(), response::Error> {
        todo!()
    }

    async fn load(&mut self, key: String) -> Result<Option<Value>, response::Error> {
        todo!()
    }

    async fn remove(&mut self, key: String) -> Result<(), response::Error> {
        todo!()
    }
}
