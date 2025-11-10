#![warn(missing_docs)]
#![doc = include_str!("../../Readme.md")]

use crate::cli::Args;
use async_channel::Receiver;
use clap::Parser;
use dsbox_core::core::error::CoreError;
use dsbox_core::core::event::Event;
use dsbox_core::core::{Builder, Core};
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
    run(args).await;
    log::logger().flush();
}

/// Starts a new [`Core`], initialized with the given [`Args`].
/// If necessary, also starts the [`Webapp`](webapp::Webapp).
async fn run(args: Args) {
    if args.interactive {
        run_webapp(&args).await;
    } else {
        run_cli(args).await
    }
}

async fn run_cli(args: Args) {
    let core = Core::builder(
        Core::split_command(&args.test_command),
        Core::make_command(args.server_command),
    )
    .interactive(false)
    .allow_lua_unsafe(args.lua_unsafe)
    .build();

    let recorder = if let Some(filename) = args.save_protocol {
        Some(spawn_protocol_recorder(core.subscribe_events(), filename).await)
    } else {
        None
    };

    core.run().await;

    if let Some(recorder) = recorder {
        recorder.await.ok();
    }
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
