use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use axum::Error;
use axum::extract::ws::{Message, WebSocket};
use serde_json::Value;
use tokio::sync::mpsc::Sender;

use json_rpc_fn::json_rpc;

use crate::core::event::Event;
use crate::core::remote_control::RemoteCommand;
use crate::webapp::json_rpc::JsonRpcDispatcher;
use crate::webapp::json_rpc::request::Request;

#[derive(Clone)]
pub struct App {
    context: Context,
    dispatcher: JsonRpcDispatcher<Context>,
}

#[derive(Clone)]
struct Context {
    remote_control: Sender<RemoteCommand>,
    storage: Arc<RwLock<HashMap<String, Value>>>,
}

impl App {
    pub fn new(remote_control: Sender<RemoteCommand>, storage: Arc<RwLock<HashMap<String, Value>>>) -> Self {
        let mut dispatcher = JsonRpcDispatcher::new();
        break_::register(&mut dispatcher);
        step::register(&mut dispatcher);
        resume::register(&mut dispatcher);
        deliver::register(&mut dispatcher);
        drop::register(&mut dispatcher);
        store::register(&mut dispatcher);
        load::register(&mut dispatcher);
        remove::register(&mut dispatcher);
        Self {
            context: Context {
                remote_control,
                storage,
            },
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
        if let Err(e) = self.dispatcher.dispatch(&mut self.context, msg.as_bytes(), &mut response).await {
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

#[json_rpc(1, "break")]
async fn break_(context: &mut Context) {
    context.remote_control.send(RemoteCommand::Break).await.ok();
}

#[json_rpc(1)]
async fn resume(context: &mut Context) {
    context.remote_control.send(RemoteCommand::Resume).await.ok();
}

#[json_rpc(1)]
async fn step(context: &mut Context) {
    context.remote_control.send(RemoteCommand::Step).await.ok();
}

#[json_rpc(1)]
async fn deliver(context: &mut Context, sent_timestamp: usize) {
    context.remote_control.send(RemoteCommand::Deliver(sent_timestamp)).await.ok();
}

#[json_rpc(1)]
async fn drop(context: &mut Context, sent_timestamp: usize) {
    context.remote_control.send(RemoteCommand::Drop(sent_timestamp)).await.ok();
}

#[json_rpc(1)]
async fn store(context: &mut Context, key: String, value: Value) {
    context.storage.write().unwrap().insert(key, value);
}

#[json_rpc(1)]
async fn load(context: &mut Context, key: String) -> Option<Value> {
    context.storage.read().unwrap().get(&key).cloned()
}

#[json_rpc(1)]
async fn remove(context: &mut Context, key: String) {
    context.storage.write().unwrap().remove(&key);
}