// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;

use app_lib::args;
use clap::Parser;
use log::LevelFilter;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut logger = env_logger::builder();
    logger.filter_level(LevelFilter::Warn);

    if cfg!(debug_assertions) {
        logger.filter_module("dsbox", LevelFilter::Trace);
    } else {
        logger.filter_module("dsbox", LevelFilter::Info);
    }
    logger.parse_default_env();
    logger.init();

    let args = args::Args::parse();
    if let Some(args::Mode::Cli(cli_args)) = args.mode {
        cli::run_cli(cli_args)
    } else {
        app_lib::run(args);
        ExitCode::SUCCESS
    }
}
