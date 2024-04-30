use std::any::Any;
use std::collections::HashMap;
use std::io::{IoSlice, IoSliceMut};
use std::path::{Path, PathBuf};

use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream, Error};
use tokio::sync::{oneshot, RwLock};
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::task::JoinHandle;
use wasi_common::{WasiCtx, WasiFile};
use wasi_common::file::FileType;
use wasi_common::tokio::WasiCtxBuilder;
use wasmtime::{Config, Engine, Linker, Module, Store, TypedFunc, UpdateDeadline};

use crate::process::{ProcessCommand, ProcessEvent};
use crate::process::io_helper::process_io_helper;

/// contains some state for launching Webassembly processes.
pub struct WasmLauncher {
    /// handle to the [`wasmtime`] [`Engine`].
    engine: Engine,
    /// cache of loaded modules, so that files need not be loaded and compiled multiple times
    /// for launching multiple processes form the same Webassembly file
    module_cache: HashMap<PathBuf, Module>,
}


/// a wrapper around a duplex stream that we can send to another task and implement WasiFile on
struct DuplexStreamFile(RwLock<DuplexStream>);

impl WasmLauncher {
    /// Initializes a new [`Engine`] for launching Webassembly processes.
    pub fn new() -> Self {
        let mut config = Config::new();
        config.async_support(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config)
            .expect("failed to initialize wasmtime engine");
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
        Self { engine, module_cache: HashMap::new() }
    }

    /// Launches a new Webassembly process from the given `path`. The Webassembly module gets passed
    /// three [`DuplexStream`]s to be used for its `stdin`, `stdout` and `stderr`.
    /// The other ends of the streams are then used to create a [`Handle`].
    /// This function is only a helper to convert any [`wasmtime::Error`] into a [`std::io::Error`] if necessary before returning.
    /// Returns the [`Handle`] and a [`Sender`] that can be used to send [`ProcessCommand`]s to the process.
    pub(super) async fn launch(&mut self, path: &Path, args: &[String], command_receiver: UnboundedReceiver<ProcessCommand>, event_sender: Sender<ProcessEvent>) -> tokio::io::Result<(JoinHandle<()>, oneshot::Receiver<()>)> {
        let (stdin, wasi_stdin) = tokio::io::duplex(1024);
        let (wasi_stdout, stdout) = tokio::io::duplex(1024);
        let (wasi_stderr, stderr) = tokio::io::duplex(1024);

        let (store, start_fn) = match self.do_launch(path, args, wasi_stdin, wasi_stdout, wasi_stderr).await {
            Ok(ret) => ret,
            Err(e) => return Err(into_io_error(e)),
        };
        let wait_child = async move {
            let exit_code = match tokio::task::spawn(async move { start_fn.call_async(store, ()).await }).await {
                Ok(Ok(())) => 0,
                _ => -1
            };
            exit_code
        };

        let (finished_tx, finished_rx) = oneshot::channel();
        Ok((tokio::task::spawn(async move {
            process_io_helper(event_sender, command_receiver, stdin, stdout, stderr, wait_child, finished_tx).await
        }), finished_rx))
    }

    /// Helper function to actually launch a Webassembly process. See ['WasmLauncher::launch'].
    async fn do_launch(&mut self, path: &Path, args: &[String], stdin: DuplexStream, stdout: DuplexStream, stderr: DuplexStream) -> wasmtime::Result<(Store<WasiCtx>, TypedFunc<(), ()>)> {
        log::trace!("launching wasm node `{}`, args: {args:?}", path.display());

        let wasi_ctx = WasiCtxBuilder::new()
            .args(args)?
            .inherit_env()?
            .stdin(DuplexStreamFile::new(stdin))
            .stdout(DuplexStreamFile::new(stdout))
            .stderr(DuplexStreamFile::new(stderr))
            .build();


        let mut linker = Linker::new(&self.engine);
        wasi_common::tokio::add_to_linker(&mut linker, |cx| cx)?;

        let module = self.load_module(path)?;
        let mut store = Store::new(&self.engine, wasi_ctx);
        store.epoch_deadline_callback(|_| {
            Ok(UpdateDeadline::Yield(1))
        });
        linker.module_async(&mut store, "", &module).await?;

        let start_fn = linker.get_default(&mut store, "")?
            .typed::<(), ()>(&mut store)?;
        Ok((store, start_fn))
    }

    /// Helper function to load a Webassembly module from a given `path`.
    fn load_module(&mut self, path: &Path) -> Result<Module, wasmtime::Error> {
        if let Some(module) = self.module_cache.get(path) {
            Ok(module.clone())
        } else {
            let module = Module::from_file(&self.engine, path)?;
            self.module_cache.insert(path.to_path_buf(), module.clone());
            Ok(module)
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

impl DuplexStreamFile {
    /// creates a new [`DuplexStreamFile`] from a [`DuplexStream`]. Conveniently puts it into a Box,
    /// because WasiCtx expects a boxed stream for its stdio
    pub fn new(stream: DuplexStream) -> Box<Self> {
        Box::new(Self(RwLock::new(stream)))
    }
}

#[wiggle::async_trait]
impl WasiFile for DuplexStreamFile {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn get_filetype(&self) -> Result<FileType, wasi_common::Error> {
        Ok(FileType::Pipe)
    }

    async fn read_vectored<'a>(&self, bufs: &mut [IoSliceMut<'a>]) -> Result<u64, wasi_common::Error> {
        let buf = bufs.iter_mut().find(|b| !b.is_empty()).map_or(&mut [][..], |b| &mut **b);
        self.0.write().await.read(buf).await
            .map(|b| b as u64)
            .map_err(|e| wasi_common::Error::from(e))
    }

    async fn write_vectored<'a>(&self, bufs: &[IoSlice<'a>]) -> Result<u64, wasi_common::Error> {
        self.0.write().await.write_vectored(bufs).await
            .map(|b| b as u64)
            .map_err(|e| wasi_common::Error::from(e))
    }

    async fn readable(&self) -> Result<(), wasi_common::Error> {
        Ok(())
    }

    async fn writable(&self) -> Result<(), wasi_common::Error> {
        Ok(())
    }
}