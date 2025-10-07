//! Command line interface for the `dsbox`.

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Option<Mode>,
    /// path to the executable or wasm-file of the server implementation, including arguments
    /// e.g. `python my_solution.py`
    #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
    pub server_command: Option<Vec<String>>,

    /// path to the executable or wasm-file of the test-case implementation
    /// e.g. `-t exercises/01-hello-world.lua`
    /// if the command should take arguments, the whole string must be quoted
    /// e.g. `-t "exercises/01-hello-world.lua --some-flag"`
    #[clap(short, long = "test")]
    pub test_command: Option<String>,

    /// allow lua test scripts to access the os library and load C modules?
    #[cfg(feature = "lua")]
    #[clap(long, default_value_t = false)]
    pub lua_unsafe: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Mode {
    /// run in non-interactive (cli) mode. No gui is started, instead the test runs once an exits
    Cli(CliArgs),
}

#[derive(Args, Debug, Clone)]
pub struct CliArgs {
    /// path to the executable or wasm-file of the server implementation, including arguments
    /// e.g. `python my_solution.py`
    #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
    pub server_command: Vec<String>,

    /// path to the executable or wasm-file of the test-case implementation
    /// e.g. `-t exercises/01-hello-world.lua`
    /// if the command should take arguments, the whole string must be quoted
    /// e.g. `-t "exercises/01-hello-world.lua --some-flag"`
    #[clap(short, long = "test")]
    pub test_command: String,

    /// after the program finished, write all events (as JSON-lines) to the specified file.
    #[clap(long)]
    pub save_protocol: Option<String>,
}
