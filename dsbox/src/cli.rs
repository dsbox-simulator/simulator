use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// path to the executable or wasm-file of the test-case implementation
    pub test_path: String,

    /// path to the executable or wasm-file of the server implementation
    #[clap(long = "servers")]
    pub server_path: String,

    /// start the simulation in interactive mode
    #[clap(long, short, default_value_t = false)]
    pub interactive: bool,

    /// listen-address for the webapp
    #[clap(default_value = "0.0.0.0")]
    pub listen_address: String,

    /// listen-port for the webapp
    #[clap(default_value_t = 8080)]
    pub port: u16,
}