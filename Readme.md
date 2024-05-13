# DsBox

a simulator for distributed systems.

## Compiling and running

`dsbox` requires a `npm`, `cargo` and ideally `rustup` to be available for building and running.
If creating wasm nodes from rust code is desired, the `wasm32-wasi` compilation target can be installed with

```shell
rustup target add wasm32-wasi
```

After that the webapp can be compiled with

```shell
npm install
npm run build
```

this will build and pack the webapp. Now to run `dsbox` and get a cli help message run:

```shell
cargo run
```

included in the project are some nodes useful for testing the program. These are in the directory `nodes` as a submodule
and can be updated/pulled using:

```shell
git submodule update --init --recursive
```

the `echo_client.py` and `echo_server.py` nodes are simple echo/reply nodes. To run the echo test use
(assumes the nodes were built in debug mode)

```shell
cargo run -- "python nodes/python/echo_client.py" --servers "python nodes/python/echo_server.py"
```

the `netsim_test` simply sends messages to itself and records their delay. To run the test use

```shell
cargo run --release -- "nodes/lua/netsim_test.lua" --servers "/dev/null"
```

(for `--servers` any path may be given, since no server is launched anyway)

to run `dsbox` in interactive mode using the webapp add the `-i` flag:

```shell
cargo run -- "python nodes/python/echo_client.py" --servers "python nodes/python/echo_server.py" -i
```

this will start a webserver on port 8080 ([http://localhost:8080]()). In debug mode, the webserver serves the files out
of the `webapp` folder directly. This means that the webapp may be changed and the website reloaded while the `dsbox` is
running. In release mode however, the webapp is embedded into the binary, so that it can run self-contained.

For more information on Nodes and how to implement them, see `nodes/Readme.md`