use clap::Parser;
use log::LevelFilter;

use crate::cli::Args;
use crate::core::Core;
use crate::core::error::CoreError;
use crate::protocol::Protocol;
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

async fn run(args: Args) -> Result<(), CoreError> {
    let core = Core::new(&args)?;

    let webapp = if args.interactive {
        Some(Webapp::run(&args, core.remote_control(), core.subscribe_events()))
    } else { None };

    let protocol = Protocol::new()
        .collect(core.subscribe_events());

    let result = tokio::task::spawn_blocking(|| core.run()).await
        .unwrap();

    if let Some(webapp) = webapp { webapp.shutdown().await; }

    protocol
        .await.unwrap()
        .write_to_file("protocol.json")
        .await.unwrap();

    result
}

// async fn run(args: Args) -> Result<(), CoreError> {
//     let core = Core::new(log_info, &args).await?;
//     let protocol = Protocol::new().collect(core.subscribe());
//
//     let webapp = if args.interactive {
//         Some(Webapp::run(&args, core.subscribe(), core.remote_control()))
//     } else { None };
//
//     core.run().await?;
//
//     if let Some(webapp) = webapp { webapp.shutdown().await; }
//     protocol.await.unwrap().write_to_file("protocol.json").await.unwrap();
//     Ok(())
// }
//
// fn log_info(id: NodeId, proc_file: &Path, line: &str) {
//     if id.is_server() {
//         log::info!("[{};{id}]: {line}", proc_file.display());
//     } else {
//         log::info!("[{}]: {line}", proc_file.display());
//     }
// }

