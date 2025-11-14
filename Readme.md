# DsBox

a simulator for distributed systems.

## Compiling and running

`dsbox` requires a `npm`, `cargo` and ideally `rustup` to be available for building and running.
If creating wasm nodes from rust code is desired, the `wasm32-wasip2` compilation target can be installed with

```shell
rustup target add wasm32-wasip2
```

The frontend for `dsbox` is written with [`tauri`](https://tauri.app). To install the tauri cli tools use

```shell
cargo install tauri-cli
```

After that the frontend dependencies must be installed with

```shell
cd webapp
npm install
```

Now to run the  `dsbox` gui

```shell
cargo tauri dev
```

To create a release executable, use
```shell
cargo tauri build --no-bundle
```

For more information on how to create nodes for tests and servers, visit the [wiki](https://git.bs.informatik.uni-siegen.de/dsbox/simulator/wiki).