//! Command line interface for the `dsbox`.

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// path to the executable or wasm-file of the server implementation, including arguments
    /// e.g. `python my_solution.py`
    #[clap(trailing_var_arg=true, allow_hyphen_values = true)]
    pub server_command: Vec<String>,

    /// path to the executable or wasm-file of the test-case implementation
    /// e.g. `-t exercises/01-hello-world.lua`
    /// if the command should take arguments, the whole string must be quoted
    /// e.g. `-t "exercises/01-hello-world.lua --some-flag"`
    #[clap(short, long="test")]
    pub test_command: String,

    /// start the simulation in interactive mode
    #[clap(short, long, default_value_t = false)]
    pub interactive: bool,

    /// listen-address for the webapp
    #[clap(short, long, default_value = "127.0.0.1")]
    pub listen_address: String,

    /// listen-port for the webapp
    #[clap(short, long, default_value_t = 8080)]
    pub port: u16,

    /// after the program finished, write all events (as JSON-lines) to the specified file
    #[clap(long)]
    pub save_protocol: Option<String>,

    /// allow lua test scripts to access the os library and load C modules?
    #[cfg(feature = "lua")]
    #[clap(long, default_value_t = false)]
    pub lua_unsafe: bool,
}