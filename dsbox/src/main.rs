#![warn(missing_docs)]
#![doc = include_str!("../../Readme.md")]

use crate::cli::Args;
use async_channel::Receiver;
use clap::Parser;
use dsbox_core::core::error::CoreError;
use dsbox_core::core::event::Event;
use dsbox_core::core::Core;
use log::LevelFilter;
use tokio::task::JoinHandle;

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
async fn run(args: Args) -> Result<(), CoreError> {
    if args.interactive {
        run_webapp(&args).await;
        Ok(())
    } else {
        run_cli(args).await
    }
}

async fn run_cli(args: Args) -> Result<(), CoreError> {
    let core = Core::new(
        Core::split_command(&args.test_command),
        Core::make_command(args.server_command),
        false,
        args.lua_unsafe,
    );

    let recorder = if let Some(filename) = args.save_protocol {
        Some(spawn_protocol_recorder(core.subscribe_events(), filename).await)
    } else {
        None
    };

    let result = core.run().await;

    if let Some(recorder) = recorder {
        recorder.await.ok();
    }
    result
}

async fn spawn_protocol_recorder(
    subscriber: Receiver<Event>,
    output_file: String,
) -> JoinHandle<()> {
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(output_file)
        .await
        .expect("failed to open protocol output file");

    tokio::task::spawn(async move {
        loop {
            let event = subscriber.recv().await;
            let Ok(event) = event else {
                break;
            };
            file.write_all(
                serde_json::to_string(&event)
                    .expect("failed to serialize event for protocol file")
                    .as_bytes(),
            )
            .await
            .expect("failed to write to protocol file");
            file.write_all(b"\n")
                .await
                .expect("failed to write to protocol file");
        }
    })
}

#[cfg(feature = "webapp")]
async fn run_webapp(args: &Args) {
    webapp::run(args).await
}

#[cfg(not(feature = "webapp"))]
async fn run_webapp(_: &Args) {
    panic!("this version of dsbox was built without webapp support")
}
