use clap::Parser;
use log::LevelFilter;
use tokio::select;

use crate::cli::Args;
use crate::core::Core;
use crate::core::error::CoreError;
use crate::protocol::ProtocolSubscriber;
use crate::webapp::Webapp;

// mod proc;
// mod node;
mod cli;
// mod select;
mod timestamp;
mod webapp;
// mod pubsub;
// mod channel;
mod process;
mod core;
mod network;
mod protocol;

#[cfg(debug_assertions)]
const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Trace;

#[cfg(not(debug_assertions))]
const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

/// Main entry point for the application. Configures logging and runs the program.
#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Warn)
        .with_module_level("dsbox", DEFAULT_LOG_LEVEL)
        .with_module_level("axum", LevelFilter::Trace)
        .init()
        .expect("failed to set logger");

    let args = Args::parse();

    if let Err(e) = run(args).await {
        log::error!("{e}")
    }

    log::logger().flush();
}

/// Starts a new [`Core`], initialized with the given [`Args`].
/// If necessary, also starts the [`Webapp`].
/// TODO: configure capturing and writing of a protocol to a file via the cli
async fn run(args: Args) -> Result<(), CoreError> {
    let core = Core::new(&args)?;

    let webapp = if args.interactive {
        Some(Webapp::run(&args, core.remote_control(), core.subscribe_events()))
    } else { None };

    let recorder = if let Some(filename) = args.save_protocol {
        Some(spawn_protocol_recorder(core.subscribe_events(), filename).await)
    } else { None };

    let result = tokio::task::spawn_blocking(|| core.run()).await
        .unwrap();

    if let Some(webapp) = webapp { webapp.shutdown().await; }
    if let Some(shutdown) = recorder {
        shutdown.send(()).await.ok();
    }
    result
}

async fn spawn_protocol_recorder(mut subscriber: ProtocolSubscriber, output_file: String) -> tokio::sync::mpsc::Sender<()> {
    use tokio::io::AsyncWriteExt;
    let (shutdown_sender, mut shutdown_receiver) = tokio::sync::mpsc::channel(1);
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(output_file).await
        .expect("failed to open protocol output file");

    tokio::task::spawn(async move {
        loop {
            select! {
            event = subscriber.recv() => {
                file.write_all(serde_json::to_string(&event).expect("failed to serialize event for protocol file").as_bytes()).await
                    .expect("failed to write to protocol file");
                file.write_all(b"\n").await
                    .expect("failed to write to protocol file");
            }
            _ = shutdown_receiver.recv() => {
                return;
            }
        }
        }
    });
    shutdown_sender
}