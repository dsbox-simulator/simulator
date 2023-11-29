use clap::Parser;
use log::LevelFilter;

use crate::cli::Args;
use crate::core::Core;
use crate::core::error::CoreError;
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

    let result = tokio::task::spawn_blocking(|| core.run()).await
        .unwrap();

    if let Some(webapp) = webapp { webapp.shutdown().await; }

    result
}

