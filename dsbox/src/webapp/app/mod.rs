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
use dsbox_core::core::Core;
use dsbox_core::protocol::ProtocolSubscriber;
use serde_json::Value;
use std::future::poll_fn;
use tokio::sync::mpsc::Sender;
use tokio::task::{JoinError, JoinHandle};

pub struct App {
    args: Args,
    remote: Sender<RemoteCommand>,
    subscriber: ProtocolSubscriber,
    core_handle: Option<JoinHandle<Result<(), CoreError>>>,
}

impl App {
    pub async fn new(
        args: Args,
        test_command: Option<String>,
        server_command: Option<String>,
    ) -> Result<Self, CoreError> {
        let core = Core::new(
            test_command.as_ref().unwrap_or(&args.test_command),
            server_command.unwrap_or_else(|| args.server_command.join(" ")),
            true,
            args.lua_unsafe,
        )
        .await?;
        let subscriber = core.subscribe_events();
        let remote = core.remote_control();
        let core_handle = tokio::task::spawn(async move { core.run().await });
        Ok(Self {
            args,
            remote,
            subscriber,
            core_handle: Some(core_handle),
        })
    }

    async fn force_restart(&mut self) -> Result<(), CoreError> {
        *self = Self::new(self.args.clone(), None, None).await?;
        Ok(())
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
            "restart" => dispatch!(restart()),
            "break" => dispatch!(break_()),
            "resume" => dispatch!(resume()),
            "step" => dispatch!(step()),
            "deliver" => dispatch!(deliver(sent_timestamp: usize)),
            "drop" => dispatch!(drop(sent_timestamp: usize)),
            "store" => dispatch!(store(key: String, value: Value)),
            "load" => dispatch!(load(key: String)),
            "remove" => dispatch!(remove(key: String)),
            _ => None,
        }
    }

    async fn handle_core_shutdown(
        &mut self,
        result: Result<Result<(), CoreError>, JoinError>,
    ) -> Result<(), CoreError> {
        match result {
            Ok(Ok(())) => {}
            Err(join_error) => log::warn!("core shut down with an error: {join_error}"),
            Ok(Err(core_error)) => log::warn!("core shut down with an error: {core_error}"),
        }
        self.force_restart().await
    }

    pub async fn run(mut self, mut socket: WebSocket) {
        loop {
            tokio::select! {
                event = self.subscriber.recv() => {
                    if let Err(e) = self.handle_event(event, &mut socket).await {
                        log::warn!("websocket error when sending event message: {e}");
                        break;
                    }
                }
                result = poll_fn(poll_optional_join_handle(&mut self.core_handle)) => {
                    if let Err(e) = self.handle_core_shutdown(result).await {
                        log::warn!("error handling core shutdown: {e}");
                        break;
                    }
                }
                message = socket.recv() => if !self.handle_socket_message(message, &mut socket).await {
                    break;
                },
            }
        }
    }

    async fn restart(&mut self) -> Result<(), response::Error> {
        // TODO: allow webapp to override test_command and server_command
        self.force_restart()
            .await
            .map_err(|e| response::Error::custom(e.to_string(), None::<()>))?;
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
