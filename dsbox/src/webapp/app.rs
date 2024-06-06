use axum::Error;
use axum::extract::ws::{Message, WebSocket};
use tokio::sync::mpsc::Sender;

use json_rpc_fn::json_rpc;

use crate::core::event::Event;
use crate::core::remote_control::RemoteCommand;
use crate::webapp::json_rpc::JsonRpcDispatcher;
use crate::webapp::json_rpc::request::Request;

#[derive(Clone)]
pub struct App {
    remote_control: Sender<RemoteCommand>,
    dispatcher: JsonRpcDispatcher<Sender<RemoteCommand>>,
}

impl App {
    pub fn new(remote_control: Sender<RemoteCommand>) -> Self {
        let mut dispatcher = JsonRpcDispatcher::new();
        pause::register(&mut dispatcher);
        step::register(&mut dispatcher);
        resume::register(&mut dispatcher);
        Self {
            remote_control,
            dispatcher,
        }
    }

    pub async fn handle_event(&mut self, event: Event, socket: &mut WebSocket) -> Result<(), Error> {
        let request = Request::notification("event".to_string(), event)
            .expect("failed to serialize jsonrpc request");
        socket.send(Message::Text(serde_json::to_string(&request)
            .expect("failed to serialize jsonrpc request"))).await
    }

    pub async fn handle_msg(&mut self, msg: String, socket: &mut WebSocket) -> Result<bool, Error> {
        let mut response = Vec::new();
        if let Err(e) = self.dispatcher.dispatch(&mut self.remote_control, msg.as_bytes(), &mut response).await {
            log::warn!("Error dispatching jsonrpc request: {e}");
            Ok(false)
        } else {
            if !response.is_empty() {
                let response_string = String::from_utf8(response).expect("expected rpc response to be a string");
                socket.send(Message::Text(response_string))
                    .await.expect("");
            }
            Ok(true)
        }
    }
}

#[json_rpc(1)]
async fn pause(remote_control: &mut Sender<RemoteCommand>) {
    remote_control.send(RemoteCommand::Pause).await.ok();
}

#[json_rpc(1)]
async fn resume(remote_control: &mut Sender<RemoteCommand>) {
    remote_control.send(RemoteCommand::Resume).await.ok();
}

#[json_rpc(1)]
async fn step(remote_control: &mut Sender<RemoteCommand>) {
    remote_control.send(RemoteCommand::Step).await.ok();
}