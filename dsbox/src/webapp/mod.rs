//! A Webapp that can be used to control the execution of the simulation interactively
//!
//! The files (html, css, js, etc.) for the webapp are either served from a folder
//! (`webapp` in the project root by default) in debug mode, or embedded directly into the binary in
//! release mode. This way the binary is self-contained.
//!
//! The [`Webapp`] serves the `index.html` file by default, and accepts [`WebSocket`][ws] connections on `/socket`.
//!
//! [ws]: https://developer.mozilla.org/en-US/docs/Web/API/WebSocket

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, ErrorKind, Read};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tower_http::trace::TraceLayer;

use crate::cli::Args;
use crate::core::remote_control::RemoteCommand;
use crate::protocol::ProtocolSubscriber;
use crate::webapp::app::App;

mod app;
mod files;
#[allow(unused)]
mod json_rpc;

/// A handle to the running webapp, mainly used to shut it down before exiting the program
pub struct Webapp {
    handle: JoinHandle<()>,
    shutdown: oneshot::Sender<()>,
}

/// State that is passed to each request handler.
/// Contains the [`Sender`] and [`ProtocolSubscriber`] for [`Event]`s and [`RemoteCommand`]s.
struct WebappState {
    app: App,
    event_subscriber: ProtocolSubscriber,
}

impl Webapp {
    /// Starts the webapp in a separate [`tokio::task`], binding it to the address and port given in the [`Args`].
    pub fn run(
        args: &Args,
        remote_control: Sender<RemoteCommand>,
        event_subscriber: ProtocolSubscriber,
    ) -> Self {
        let (tx, rx) = oneshot::channel();
        let listen_address =
            SocketAddr::new(IpAddr::from_str(&args.listen_address).unwrap(), args.port);
        Self {
            handle: tokio::spawn(async move {
                let storage = Self::load_storage().unwrap_or_else(|e| {
                    log::warn!("failed to load webapp storage: {e}");
                    HashMap::new()
                });
                let storage = Arc::new(RwLock::new(storage));
                // build our application with a route
                let router = Self::build_router().with_state(WebappState {
                    app: App::new(remote_control, storage.clone()),
                    event_subscriber,
                });

                // run our app with hyper
                // `axum::Server` is a re-export of `hyper::Server`
                log::info!("listening on http://{listen_address}");
                let listener = tokio::net::TcpListener::bind(listen_address)
                    .await
                    .expect("failed to bind tcp listener");
                let server = axum::serve(listener, router);
                let server = server.with_graceful_shutdown(async move {
                    rx.await.ok();
                });
                match server.await {
                    Ok(()) => log::info!("web server shutdown successful"),
                    Err(e) => log::warn!("web server shutdown error: {e}"),
                }
                let storage = storage.read().unwrap();
                if let Err(e) = Self::save_storage(&*storage) {
                    log::warn!("failed to save webapp storage: {e}");
                }
            }),
            shutdown: tx,
        }
    }

    /// Sends a shutdown notification to the webapp task, and waits for it to terminate.
    pub async fn shutdown(self) {
        self.shutdown.send(()).ok();
        self.handle.await.ok();
    }

    /// Helper function to build all routes that the webapp serves.
    fn build_router() -> Router<WebappState> {
        Router::new()
            .route("/", get(serve_static))
            .route("/{*path}", get(serve_static))
            .route("/socket", get(socket))
            .layer(TraceLayer::new_for_http().on_request(()).on_response(()))
    }

    const STORAGE_FILE: &'static str = ".dsbox_storage.json";
    fn load_storage() -> std::io::Result<HashMap<String, Value>> {
        let file = match File::options().read(true).open(Self::STORAGE_FILE) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(e) => return Err(e),
        };
        serde_json::from_reader(&mut BufReader::new(file)).map_err(Into::into)
    }

    fn save_storage(storage: &HashMap<String, Value>) -> std::io::Result<()> {
        let file = File::options()
            .write(true)
            .create(true)
            .open(Self::STORAGE_FILE)?;
        serde_json::to_writer(&mut BufWriter::new(file), storage).map_err(Into::into)
    }
}

/// Request handler that is called for all static file requests. If `path` is `None` it serves `index.html`
/// otherwise it looks for the specified file and serves that, or returns a 404 if it is not found.
async fn serve_static(
    path: Option<Path<String>>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    let path = if let Some(Path(path)) = path {
        path
    } else {
        String::from("index.html")
    };
    let can_decompress = headers
        .get("Accept-Encoding")
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
            response
                .headers_mut()
                .insert("Content-Encoding", HeaderValue::from_static("gzip"));
            response
        } else {
            let mut reader = flate2::read::GzDecoder::<&[u8]>::new(file.data.as_ref());
            let mut decoded = Vec::new();
            reader.read_to_end(&mut decoded).unwrap();
            decoded.into_response()
        };
        response.headers_mut().insert(
            "Content-Type",
            HeaderValue::from_str(file.mime_type.as_ref()).unwrap(),
        );
        Ok(response)
    } else {
        Err((StatusCode::NOT_FOUND, format!("`{path}` not found")))
    }
}

/// Request handler that is called when a new [`WebSocket`][ws] connection is being established.
/// Finalizes the connection using the [`socket_handler`] as a handler for the [`WebSocket`][ws] connection.
///
/// [ws]: https://developer.mozilla.org/en-US/docs/Web/API/WebSocket
async fn socket(state: State<WebappState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| socket_handler(state, socket))
}

/// Handler for a [`WebSocket`][ws] connection. Listens for messages and attempts to send corresponding
/// [`RemoteCommand`]s to the [`Core`](crate::core::Core), and sends published [`Event`](crate::core::event::Event)s
/// from the [`Core`](crate::core::Core) into the socket.
///
/// [ws]: https://developer.mozilla.org/en-US/docs/Web/API/WebSocket
async fn socket_handler(State(mut state): State<WebappState>, mut socket: WebSocket) {
    loop {
        tokio::select! {
            event = state.event_subscriber.recv() => {
                if let Err(e) = state.app.handle_event(event, &mut socket).await {
                    log::warn!("websocket error when sending event message: {e}");
                    break;
                }
            }
            command = socket.recv() => {
                match command {
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Text(message))) => if !state.app.handle_msg(String::from_utf8_lossy(message.as_bytes()), &mut socket).await
                        .expect("failed to handle message") { break; },
                    Some(Ok(_)) => panic!("unknown message type"),
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

impl Clone for WebappState {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            event_subscriber: self.event_subscriber.resubscribe(),
        }
    }
}
