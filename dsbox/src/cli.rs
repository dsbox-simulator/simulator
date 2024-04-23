//! Command line interface for the `dsbox`.

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// path to the executable or wasm-file of the test-case implementation
    pub test_command: String,

    /// path to the executable or wasm-file of the server implementation
    #[clap(long = "servers")]
    pub server_command: String,

    /// start the simulation in interactive mode
    #[clap(long, short, default_value_t = false)]
    pub interactive: bool,

    /// listen-address for the webapp
    #[clap(default_value = "0.0.0.0")]
    pub listen_address: String,

    /// listen-port for the webapp
    #[clap(default_value_t = 8080)]
    pub port: u16,

    /// after the program finished, write all events (as JSON-lines) to the specified file
    #[clap(long)]
    pub save_protocol: Option<String>
}