use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use axum::extract::{Path, State, WebSocketUpgrade};
use axum::extract::ws::{Message, WebSocket};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use axum::routing::get;
use crossbeam_channel::Sender;
use tokio::sync::{broadcast, oneshot};
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

use crate::cli::Args;
use crate::core::event::Event;
use crate::core::remote_control::RemoteCommand;

mod files;

pub struct Webapp {
    handle: JoinHandle<()>,
    shutdown: oneshot::Sender<()>,
}

struct WebappState {
    remote_control: Sender<RemoteCommand>,
    event_receiver: broadcast::Receiver<Event>,
}

impl Webapp {
    pub fn run(args: &Args, remote_control: Sender<RemoteCommand>, event_receiver: broadcast::Receiver<Event>) -> Self {
        let (tx, rx) = oneshot::channel();
        let listen_address = SocketAddr::new(IpAddr::from_str(&args.listen_address).unwrap(), args.port);
        Self {
            handle: tokio::spawn(async move {
                // build our application with a route
                let router = Self::build_router()
                    .with_state(WebappState { remote_control, event_receiver });

                // run our app with hyper
                // `axum::Server` is a re-export of `hyper::Server`
                log::info!("listening on {listen_address}");
                let server = axum::Server::bind(&listen_address)
                    .serve(router.into_make_service());

                let server = server.with_graceful_shutdown(async { rx.await.ok(); });
                if let Err(e) = server.await {
                    log::warn!("web server terminated with an error: {e}")
                }
            }),
            shutdown: tx,
        }
    }

    pub async fn shutdown(self) {
        self.shutdown.send(()).ok();
        self.handle.await.ok();
    }

    fn build_router() -> Router<WebappState> {
        Router::new()
            .route("/", get(serve_static))
            .route("/*path", get(serve_static))
            .route("/socket", get(socket))
    }
}

async fn serve_static(path: Option<Path<String>>, headers: HeaderMap) -> Result<Response, (StatusCode, String)> {
    let path = if let Some(Path(path)) = path { path } else { String::from("index.html") };
    let can_decompress = headers.get("Accept-Encoding")
        .map(|h| h.to_str().unwrap().contains("gzip"))
        .unwrap_or(false);
    let file = match files::lookup(&path).await {
        Ok(file) => file,
        Err(io) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, io.to_string()));
        }
    };
    if let Some(file) = file {
        let mut response = if !file.compressed {
            file.data.into_response()
        } else if can_decompress {
            let mut response = file.data.into_response();
            response.headers_mut().insert("Content-Encoding", HeaderValue::from_static("gzip"));
            response
        } else {
            let mut reader = flate2::read::GzDecoder::<&[u8]>::new(file.data.as_ref());
            let mut decoded = Vec::new();
            reader.read_to_end(&mut decoded).unwrap();
            decoded.into_response()
        };
        response.headers_mut().insert("Content-Type", HeaderValue::from_str(file.mime_type.as_ref()).unwrap());
        Ok(response)
    } else {
        Err((StatusCode::NOT_FOUND, format!("`{path}` not found")))
    }
}

async fn socket(state: State<WebappState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| socket_handler(state, socket))
}

async fn socket_handler(State(mut state): State<WebappState>, mut socket: WebSocket) {
    loop {
        tokio::select! {
            event = state.event_receiver.recv() => {
                match event {
                    Ok(event) => if let Err(e) = socket.send(Message::Text(serde_json::to_string(&event).unwrap())).await {
                        log::warn!("websocket error when sending event message: {e}");
                        break;
                    },
                    Err(RecvError::Lagged(_)) => log::warn!("webapp lags behind core events"),
                    Err(RecvError::Closed) => break,
                }
            }
            command = socket.recv() => {
                match command {
                    Some(Ok(ws_msg)) => if !handle_msg(ws_msg, &mut state.remote_control).await { break; }
                    Some(Err(e)) => {
                        log::warn!("websocket error: {e}");
                        break;
                    }
                    None => break,
                }
            }
        }
    }
}

async fn handle_msg(msg: Message, remote_control: &mut Sender<RemoteCommand>) -> bool {
    if matches!(msg, Message::Close(_)) { return false; }
    if let Message::Text(txt) = msg {
        match txt.as_str() {
            "pause" => remote_control.send(RemoteCommand::Pause).is_ok(),
            "resume" => remote_control.send(RemoteCommand::Resume).is_ok(),
            "step" => remote_control.send(RemoteCommand::Step).is_ok(),
            cmd => {
                log::warn!("unknown core command: `{cmd}`");
                true
            }
        }
    } else {
        true
    }
}

impl Clone for WebappState {
    fn clone(&self) -> Self {
        Self {
            remote_control: self.remote_control.clone(),
            event_receiver: self.event_receiver.resubscribe(),
        }
    }
}