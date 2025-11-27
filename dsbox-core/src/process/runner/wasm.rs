use crate::command::ExecutableCommand;
use crate::process::ProcessEvent;
use crate::process::runner::{CommandReceiver, Runner, EventSender, io_helper};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::Path;
use tokio::io::{ReadHalf, SimplexStream, WriteHalf};
use wasmtime::component::Component;
use wasmtime::{
    CodeBuilder, CodeHint, Config, Engine, Linker, Module, Store, TypedFunc, component,
};
use wasmtime_wasi::cli::{AsyncStdinStream, AsyncStdoutStream};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::{WasiCtxBuilder, p2, p3};

pub struct WasmRunner {
    /// handle to the [`wasmtime`] [`Engine`].
    engine: Engine,
    /// cache of loaded and compile wasm files, so that they do not need to be loaded and compiled
    /// multiple times for launching multiple processes form the same file
    wasm_cache: HashMap<String, LoadedWasm>,
}

#[derive(Clone)]
enum LoadedWasm {
    Module(Module),
    Component(Component),
}

enum EntryPoint {
    P1(TypedFunc<(), ()>),
    P2(p2::bindings::Command),
    P3(p3::bindings::Command),
}

impl WasmRunner {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.async_support(true);
        let engine = Engine::new(&config).expect("failed to initialize wasmtime engine");
        Self {
            engine,
            wasm_cache: HashMap::new(),
        }
    }

    fn load_file(&mut self, path: String) -> wasmtime::Result<LoadedWasm> {
        match self.wasm_cache.entry(path) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => {
                let mut builder = CodeBuilder::new(&self.engine);
                builder.wasm_binary_or_text_file(Path::new(entry.key()))?;
                let loaded = match builder.hint() {
                    Some(CodeHint::Module) => LoadedWasm::Module(builder.compile_module()?),
                    Some(CodeHint::Component) => {
                        LoadedWasm::Component(builder.compile_component()?)
                    }
                    None => return Err(wasmtime::Error::msg("could not determine wasm file type")),
                };
                drop(builder);
                Ok(entry.insert(loaded).clone())
            }
        }
    }
}

impl Runner for WasmRunner {
    fn run(
        &mut self,
        command: ExecutableCommand,
        sender: EventSender,
        receiver: CommandReceiver,
    ) -> impl Future<Output = i32> + 'static {
        let engine = self.engine.clone();
        let wasm = self.load_file(command.program);
        let (mut store, stdin, stdout, stderr) = new_store(&engine, command.args);
        async move {
            let wasm = match wasm {
                Ok(wasm) => wasm,
                Err(e) => {
                    sender.send(ProcessEvent::Log(e.to_string())).await.ok();
                    return -1;
                }
            };
            let entry_point = match instantiate_wasm(wasm, &mut store, engine).await {
                Ok(entry_point) => entry_point,
                Err(e) => {
                    sender.send(ProcessEvent::Log(e.to_string())).await.ok();
                    return -1;
                }
            };
            let child = tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(run_wasm(entry_point, store))
            });
            io_helper::io_helper(sender, receiver, stdin, stdout, stderr, async move {
                child.await.unwrap_or(1)
            })
            .await
        }
    }
}

fn new_store(
    engine: &Engine,
    args: Vec<String>,
) -> (
    Store<WasiP1Ctx>,
    WriteHalf<SimplexStream>,
    ReadHalf<SimplexStream>,
    ReadHalf<SimplexStream>,
) {
    let (wasi_stdin, stdin) = tokio::io::simplex(1024);
    let (stdout, wasi_stdout) = tokio::io::simplex(1024);
    let (stderr, wasi_stderr) = tokio::io::simplex(1024);

    let cleaned_env = std::env::vars()
        .filter(|(name, _)| name.starts_with("DSBOX_"))
        .collect::<Vec<_>>();

    let ctx = WasiCtxBuilder::new()
        .args(&args)
        .envs(&cleaned_env)
        .stdin(AsyncStdinStream::new(wasi_stdin))
        .stdout(AsyncStdoutStream::new(1024, wasi_stdout))
        .stderr(AsyncStdoutStream::new(1024, wasi_stderr))
        .build_p1();

    (Store::new(engine, ctx), stdin, stdout, stderr)
}

async fn instantiate_wasm(
    wasm: LoadedWasm,
    store: &mut Store<WasiP1Ctx>,
    engine: Engine,
) -> wasmtime::Result<EntryPoint> {
    match wasm {
        LoadedWasm::Module(module) => instantiate_module(&module, store, &engine).await,
        LoadedWasm::Component(component) => instantiate_component(&component, store, &engine).await,
    }
}

async fn run_wasm(entry_point: EntryPoint, mut store: Store<WasiP1Ctx>) -> i32 {
    let result = match entry_point {
        EntryPoint::P1(start_fn) => start_fn.call_async(store, ()).await.map(|_| ()),
        EntryPoint::P2(command) => command.wasi_cli_run().call_run(store).await.map(|_| ()),
        EntryPoint::P3(command) => store
            .run_concurrent(async |store| command.wasi_cli_run().call_run(store).await)
            .await
            .map(|_| ()),
    };
    match result {
        Ok(_) => 0,
        Err(e) => exit_code(e),
    }
}

async fn instantiate_module(
    module: &Module,
    store: &mut Store<WasiP1Ctx>,
    engine: &Engine,
) -> wasmtime::Result<EntryPoint> {
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p1::add_to_linker_async(&mut linker, |ctx| ctx)?;
    linker.module_async(&mut *store, "", module).await?;
    Ok(EntryPoint::P1(
        linker
            .get_default(&mut *store, "")?
            .typed::<(), ()>(store)?,
    ))
}

async fn instantiate_component(
    component: &Component,
    store: &mut Store<WasiP1Ctx>,
    engine: &Engine,
) -> wasmtime::Result<EntryPoint> {
    let mut linker = component::Linker::new(&engine);
    let instance = linker.instantiate_async(&mut *store, component).await?;
    p2::add_to_linker_async(&mut linker)?;
    p3::add_to_linker(&mut linker)?;

    if let Ok(command) = p3::bindings::Command::new(&mut *store, &instance) {
        Ok(EntryPoint::P3(command))
    } else {
        Ok(EntryPoint::P2(p2::bindings::Command::new(
            &mut *store,
            &instance,
        )?))
    }
}

/// attempts to infer an exit code from a [`wasmtime::Error`].
/// Taken from [https://docs.rs/wasi-common/latest/wasi_common/fn.maybe_exit_on_error.html]()
pub fn exit_code(e: wasmtime::Error) -> i32 {
    // If a specific WASI error code was requested then that's
    // forwarded through to the process here
    if let Some(exit) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
        exit.0
    } else if e.is::<wasmtime::Trap>() {
        // If the program exited because of a trap, return an error code
        // to the outside environment indicating a more severe problem
        // than a simple failure.
        if cfg!(unix) {
            // On Unix, return the error code of an abort.
            128 + libc::SIGABRT
        } else if cfg!(windows) {
            // On Windows, return 3.
            // https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/abort?view=vs-2019
            3
        } else {
            1
        }
    } else {
        0
    }
}
