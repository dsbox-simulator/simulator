
# DsBox
a simulator for distributed systems.

## compiling

`dsbox` requires a `npm`, `cargo` and ideally `rustup` to be available for building and running. 
Additionaly a nightly version of rust must be used. In order to enable nightly using `rustup` use:
```shell
rustup override nightly
```
in the project directory. After that the webapp can be compiled with 
```shell
npm install
npm run build
```
this will build and pack the webapp. Now to run `dsbox` and get a cli help message run:
```shell
cargo run
```
included in the project are three types of nodes useful for testing the program. These are in the directory `nodes` and 
can be built using:
```shell
cargo build --workspace --exclude dsbox
```
the `echo_client` and `echo_server` nodes are simple echo/reply nodes. To run the echo test use 
(assumes the nodes were built in debug mode)
```shell
cargo run -- "target/debug/echo_client" --servers "target/wasm32-wasi/debug/echo_server.wasm"
```
the `netsim_test` simply sends messages to itself and records their delay. To run the test use
```shell
cargo build --release --workspace
cargo run --release -- "target/release/netsim_test" --servers "/dev/null"
```
(for `--servers` any path may be given, since no server is launched anyway)

to run `dsbox` in interactive mode using the webapp add the `-i` flag:
```shell
cargo run -- "target/debug/echo_client" --servers "target/wasm32-wasi/debug/echo_server.wasm" -i
```
this will start a webserver on port 8080 ([http://localhost:8080]()). In debug mode, the webserver serves the files out 
of the `webapp` folder directly. This means that the webapp may be changed and the website reloaded while the `dsbox` is 
running. In release mode however, the webapp is embedded into the binary, so that it can run self-contained. 