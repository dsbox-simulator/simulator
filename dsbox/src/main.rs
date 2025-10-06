#![warn(missing_docs)]
#![doc = include_str!("../../Readme.md")]

use crate::cli::Args;
use clap::Parser;
use dsbox_core::core::error::CoreError;
use dsbox_core::core::Core;
use dsbox_core::protocol::ProtocolSubscriber;
use log::LevelFilter;
use tokio::select;
use tokio::sync::mpsc::Sender;
use dsbox_core::core::remote_control::RemoteCommand;

mod cli;
#[cfg(feature = "webapp")]
mod webapp;

/// Main entry point for the application. Configures logging and runs the program.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut logger = env_logger::builder();
    logger.filter_level(LevelFilter::Warn);

    if cfg!(debug_assertions) {
        logger
            .filter_module("dsbox", LevelFilter::Trace)
            .filter_module("tower_http", LevelFilter::Debug)
            .filter_module("axum", LevelFilter::Debug);
    } else {
        logger.filter_module("dsbox", LevelFilter::Info);
    }
    logger.parse_default_env();
    logger.init();

    let args = Args::parse();
    if let Err(e) = run(args).await {
        log::error!("{e}")
    }

    log::logger().flush();
}

/// Starts a new [`Core`], initialized with the given [`Args`].
/// If necessary, also starts the [`Webapp`](webapp::Webapp).
/// TODO: configure capturing and writing of a protocol to a file via the cli
async fn run(args: Args) -> Result<(), CoreError> {
    let core = Core::new(
        &args.test_command,
        args.server_command.join(" "),
        args.interactive,
        args.lua_unsafe,
    )
    .await?;

    let webapp = if args.interactive {
        Some(run_webapp(
            &args,
            core.remote_control(),
            core.subscribe_events(),
        ))
    } else {
        None
    };

    let recorder = if let Some(filename) = args.save_protocol {
        Some(spawn_protocol_recorder(core.subscribe_events(), filename).await)
    } else {
        None
    };

    let result = core.run().await;

    if let Some(webapp) = webapp {
        shutdown_webapp(webapp).await;
    }

    if let Some(shutdown) = recorder {
        shutdown.send(()).await.ok();
    }
    result
}

async fn spawn_protocol_recorder(
    mut subscriber: ProtocolSubscriber,
    output_file: String,
) -> Sender<()> {
    use tokio::io::AsyncWriteExt;
    let (shutdown_sender, mut shutdown_receiver) = tokio::sync::mpsc::channel(1);
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(output_file)
        .await
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

#[cfg(feature = "webapp")]
fn run_webapp(
    args: &Args,
    remote_control: Sender<RemoteCommand>,
    event_subscriber: ProtocolSubscriber,
) -> webapp::Webapp {
    webapp::Webapp::run(args, remote_control, event_subscriber)
}

#[cfg(not(feature = "webapp"))]
fn run_webapp(_: &Args, _: Sender<RemoteCommand>, _: ProtocolSubscriber) -> () {
    panic!("this version of dsbox was built without webapp support")
}

#[cfg(feature = "webapp")]
async fn shutdown_webapp(webapp: webapp::Webapp) {
    webapp.shutdown().await;
}

#[cfg(not(feature = "webapp"))]
async fn shutdown_webapp(_: ()) {
    panic!("this version of dsbox was built without webapp support")
}
