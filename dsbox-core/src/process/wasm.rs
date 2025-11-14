use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tokio::io::{Error, ReadHalf, SimplexStream, WriteHalf};
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use wasmtime::component::Component;
use wasmtime::{
    CodeBuilder, CodeHint, Config, Engine, Linker, Module, Store, TypedFunc, UpdateDeadline,
    component,
};
use wasmtime_wasi::cli::{AsyncStdinStream, AsyncStdoutStream};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::{WasiCtxBuilder, p2, p3};

use crate::process::io_helper::process_io_helper;
use crate::process::{ProcessCommand, ProcessEvent};

/// contains some state for launching Webassembly processes.
pub struct WasmLauncher {
    /// handle to the [`wasmtime`] [`Engine`].
    engine: Engine,
    /// cache of loaded and compile wasm files, so that they do not need to be loaded and compiled
    /// multiple times for launching multiple processes form the same file
    wasm_cache: HashMap<PathBuf, LoadedWasm>,
}

enum LoadedWasm {
    Module(Module),
    Component(Component),
}

enum StartFn {
    P1(TypedFunc<(), ()>),
    P2(p2::bindings::Command),
    P3(component::Instance, p3::bindings::Command),
}

impl WasmLauncher {
    /// Initializes a new [`Engine`] for launching Webassembly processes.
    pub fn new() -> Self {
        let mut config = Config::new();
        config.async_support(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).expect("failed to initialize wasmtime engine");
        // spawn a thread to increment the epoch counter every 200ms so that no wasm module can entirely block the runtime
        {
            let engine = engine.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    engine.increment_epoch();
                }
            });
        }
        Self {
            engine,
            wasm_cache: HashMap::new(),
        }
    }

    /// Launches a new Webassembly process from the given `path`. The Webassembly module gets passed
    /// three [`DuplexStream`]s to be used for its `stdin`, `stdout` and `stderr`.
    /// This function is only a helper to convert any [`wasmtime::Error`] into a [`std::io::Error`] if necessary before returning.
    pub(super) async fn launch(
        &mut self,
        path: &Path,
        args: &[String],
        command_receiver: UnboundedReceiver<ProcessCommand>,
        event_sender: Sender<ProcessEvent>,
    ) -> tokio::io::Result<(JoinHandle<()>, oneshot::Receiver<()>)> {
        let (wasi_stdin, stdin) = tokio::io::simplex(1024);
        let (stdout, wasi_stdout) = tokio::io::simplex(1024);
        let (stderr, wasi_stderr) = tokio::io::simplex(1024);

        let (mut store, start_fn) = match self
            .do_launch(path, args, wasi_stdin, wasi_stdout, wasi_stderr)
            .await
        {
            Ok(ret) => ret,
            Err(e) => return Err(into_io_error(e)),
        };
        let wait_child = async move {
            let run_task = async move {
                match start_fn {
                    StartFn::P1(start_fn) => match start_fn.call_async(store, ()).await {
                        Ok(()) => 0,
                        Err(_) => -1,
                    },
                    StartFn::P2(command) => match command.wasi_cli_run().call_run(store).await {
                        Ok(Ok(())) => 0,
                        _ => -1,
                    },
                    StartFn::P3(instance, command) => {
                        let result = instance
                            .run_concurrent(&mut store, async move |store| {
                                command.wasi_cli_run().call_run(store).await
                            })
                            .await;
                        match result {
                            Ok(Ok(Ok(()))) => 0,
                            _ => -1,
                        }
                    }
                }
            };
            let exit_code = match tokio::task::spawn(run_task).await {
                Ok(exit_code) => exit_code,
                _ => -1,
            };
            exit_code
        };

        let (finished_tx, finished_rx) = oneshot::channel();
        Ok((
            tokio::task::spawn(async move {
                process_io_helper(
                    event_sender,
                    command_receiver,
                    stdin,
                    stdout,
                    stderr,
                    wait_child,
                    finished_tx,
                )
                .await
            }),
            finished_rx,
        ))
    }

    /// Helper function to actually launch a Webassembly process. See ['WasmLauncher::launch'].
    async fn do_launch(
        &mut self,
        path: &Path,
        args: &[String],
        stdin: ReadHalf<SimplexStream>,
        stdout: WriteHalf<SimplexStream>,
        stderr: WriteHalf<SimplexStream>,
    ) -> wasmtime::Result<(Store<WasiP1Ctx>, StartFn)> {
        log::trace!("launching wasm node `{}`, args: {args:?}", path.display());

        let loaded = if let Some(loaded) = self.wasm_cache.get(path) {
            loaded
        } else {
            let mut builder = CodeBuilder::new(&self.engine);
            builder.wasm_binary_or_text_file(path)?;
            let loaded = match builder.hint() {
                Some(CodeHint::Module) => LoadedWasm::Module(builder.compile_module()?),
                Some(CodeHint::Component) => LoadedWasm::Component(builder.compile_component()?),
                None => return Err(wasmtime::Error::msg("could not determine wasm file type")),
            };
            self.wasm_cache.insert(path.to_path_buf(), loaded);
            self.wasm_cache.get(path).unwrap()
        };

        let cleaned_env = std::env::vars()
            .filter(|(name, _)| name.starts_with("DSBOX_"))
            .collect::<Vec<_>>();

        let ctx = WasiCtxBuilder::new()
            .args(args)
            .envs(&cleaned_env)
            .stdin(AsyncStdinStream::new(stdin))
            .stdout(AsyncStdoutStream::new(1024, stdout))
            .stderr(AsyncStdoutStream::new(1024, stderr))
            .build_p1();
        let mut store = Store::new(&self.engine, ctx);
        store.epoch_deadline_callback(|_| Ok(UpdateDeadline::Yield(1)));

        let start_fn = match loaded {
            LoadedWasm::Module(module) => self.launch_module(module, &mut store).await?,
            LoadedWasm::Component(component) => {
                self.launch_component(component, &mut store).await?
            }
        };
        Ok((store, start_fn))
    }

    async fn launch_module(
        &self,
        module: &Module,
        store: &mut Store<WasiP1Ctx>,
    ) -> wasmtime::Result<StartFn> {
        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::p1::add_to_linker_async(&mut linker, |ctx| ctx)?;

        linker.module_async(&mut *store, "", module).await?;

        let start_fn = linker
            .get_default(&mut *store, "")?
            .typed::<(), ()>(store)?;
        Ok(StartFn::P1(start_fn))
    }

    async fn launch_component(
        &self,
        component: &Component,
        store: &mut Store<WasiP1Ctx>,
    ) -> wasmtime::Result<StartFn> {
        let mut linker = component::Linker::new(&self.engine);
        let instance = linker.instantiate_async(&mut *store, component).await?;
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        wasmtime_wasi::p3::add_to_linker(&mut linker)?;

        if let Ok(command) = wasmtime_wasi::p3::bindings::Command::new(&mut *store, &instance) {
            Ok(StartFn::P3(instance, command))
        } else {
            let command = wasmtime_wasi::p2::bindings::Command::new(&mut *store, &instance)?;
            Ok(StartFn::P2(command))
        }
    }
}

/// Converts a [`wasmtime::Error`] into a [`Error`].
fn into_io_error(error: wasmtime::Error) -> Error {
    for e in error.chain() {
        if let Some(e) = e.downcast_ref::<Error>() {
            return Error::new(e.kind(), e.to_string());
        }
    }
    return Error::new(std::io::ErrorKind::Other, error.to_string());
}
