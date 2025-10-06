//! A Webapp that can be used to control the execution of the simulation interactively
//!
//! The files (html, css, js, etc.) for the webapp are either served from a folder
//! (`webapp` in the project root by default) in debug mode, or embedded directly into the binary in
//! release mode. This way the binary is self-contained.
//!
//! The [`Webapp`] serves the `index.html` file by default, and accepts [`WebSocket`][ws] connections on `/socket`.
//!
//! [ws]: https://developer.mozilla.org/en-US/docs/Web/API/WebSocket

use crate::cli::Args;
use crate::webapp::app::App;
use axum::extract::ws::WebSocket;
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
use tower_http::trace::TraceLayer;

mod app;
mod files;
mod json_rpc;

pub async fn run(args: &Args) {
    let args = args.clone();
    let listen_address =
        SocketAddr::new(IpAddr::from_str(&args.listen_address).unwrap(), args.port);
    let storage = load_storage().unwrap_or_else(|e| {
        log::warn!("failed to load webapp storage: {e}");
        HashMap::new()
    });
    let storage = Arc::new(RwLock::new(storage));
    // build our application with a route
    let router = build_router().with_state(args);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log::info!("listening on http://{listen_address}");
    let listener = tokio::net::TcpListener::bind(listen_address)
        .await
        .expect("failed to bind tcp listener");
    let server = axum::serve(listener, router);
    match server.await {
        Ok(()) => log::info!("web server shutdown successful"),
        Err(e) => log::warn!("web server shutdown error: {e}"),
    }
    let storage = storage.read().unwrap();
    if let Err(e) = save_storage(&*storage) {
        log::warn!("failed to save webapp storage: {e}");
    }
}

/// Helper function to build all routes that the webapp serves.
fn build_router() -> Router<Args> {
    Router::new()
        .route("/", get(serve_static))
        .route("/{*path}", get(serve_static))
        .route("/socket", get(socket))
        .layer(TraceLayer::new_for_http().on_request(()).on_response(()))
}

const STORAGE_FILE: &'static str = ".dsbox_storage.json";
fn load_storage() -> std::io::Result<HashMap<String, Value>> {
    let file = match File::options().read(true).open(STORAGE_FILE) {
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
        .open(STORAGE_FILE)?;
    serde_json::to_writer(&mut BufWriter::new(file), storage).map_err(Into::into)
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
async fn socket(state: State<Args>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| socket_handler(state, socket))
}

/// Handler for a [`WebSocket`][ws] connection. Listens for messages and attempts to send corresponding
/// [`RemoteCommand`]s to the [`Core`](crate::core::Core), and sends published [`Event`](crate::core::event::Event)s
/// from the [`Core`](crate::core::Core) into the socket.
///
/// [ws]: https://developer.mozilla.org/en-US/docs/Web/API/WebSocket
async fn socket_handler(State(args): State<Args>, socket: WebSocket) {
    let app = App::new(args.clone(), None, None)
        .await
        .expect("failed to start core");
    app.run(socket).await;
}