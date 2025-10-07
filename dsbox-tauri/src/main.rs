// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app_lib::cli;
use async_channel::Receiver;
use clap::Parser;
use dsbox_core::core::error::CoreError;
use dsbox_core::core::event::Event;
use dsbox_core::core::Core;
use log::LevelFilter;
use tokio::task::JoinHandle;

fn main() {
    let args = cli::Cli::parse();
    if let Some(cli::Mode::Cli(cli_args)) = args.mode {
        run_cli(cli_args, args.lua_unsafe);
    } else {
        app_lib::run(args);
    }
}

fn run_cli(args: cli::CliArgs, allow_lua_unsafe: bool) {
    let mut logger = env_logger::builder();
    logger.filter_level(LevelFilter::Warn);

    if cfg!(debug_assertions) {
        logger.filter_module("dsbox", LevelFilter::Trace);
    } else {
        logger.filter_module("dsbox", LevelFilter::Info);
    }
    logger.parse_default_env();
    logger.init();

    if let Err(e) = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(run_dsbox(args, allow_lua_unsafe))
    {
        log::error!("{e}");
    }
}

async fn run_dsbox(args: cli::CliArgs, allow_lua_unsafe: bool) -> Result<(), CoreError> {
    let core = Core::new(
        Some(args.test_command),
        args.server_command.join(" "),
        false,
        allow_lua_unsafe,
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
