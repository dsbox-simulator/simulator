use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::io::{Error, IoSlice, IoSliceMut, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;
use wasi_common::file::FileType;
use wasi_common::WasiFile;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

use crate::process::{ProcessCommand, ProcessEvent};
use crate::process::handle::Handle;

/// contains some state for launching Webassembly processes.
pub struct WasmLauncher {
    /// handle to the [`wasmtime`] [`Engine`].
    engine: Engine,
    /// cache of loaded modules, so that files need not be loaded and compiled multiple times
    /// for launching multiple processes form the same Webassembly file
    module_cache: HashMap<PathBuf, Module>,
}

/// An in-memory [`Read`]er. Is always paired with a corresponding [`MemoryWriter`]
/// It implements both [`Read`] as well as [`WasiFile`] (read-only) so that it can be used in the
/// Webassembly module as `stdin` and in the handling threads/tasks as the other end of `stdout` and `stderr`.
struct MemoryReader {
    /// reference to the actual [`MemoryStream`] that is shared between this [`MemoryReader`] and its corresponding [`MemoryWriter`]
    inner: Arc<MemoryStream>,
}

/// An in-memory [`Write`]r. Is always paired with a corresponding [`MemoryReader`]
/// It implements both [`Write`] as well as [`WasiFile`] (write-only) so that it can be used in the
/// Webassembly module as `stdout` and `stderr` and in the handling threads/tasks as the other end of `stdin`.
struct MemoryWriter {
    /// reference to the actual [`MemoryStream`] that is shared between this [`MemoryWriter`] and its corresponding [`MemoryReader`]
    inner: Arc<MemoryStream>,
}

/// An in-memory stream, that can be used to pass bytes between a [`MemoryReader`] and a [`MemoryWriter`]
struct MemoryStream {
    /// the actual bytes currently written into, but not read out of, this stream
    data: Mutex<VecDeque<u8>>,
    /// condition variable to notify the reader, that bytes are available to read (or that the writer has been dropped).
    data_available: Condvar,
    /// set to `true` when the writer drops.
    writer_closed: AtomicBool,
}

impl WasmLauncher {
    /// Initializes a new [`Engine`] for launching Webassembly processes.
    pub fn new() -> Self {
        let config = Config::new();
        Self { engine: Engine::new(&config).unwrap(), module_cache: HashMap::new() }
    }

    /// Launches a new Webassembly process from the given `path`. The Webassembly module gets passed
    /// two [`MemoryWriter`]s and a [`MemoryReader`] to be used for its `stdin`, `stdout` and `stderr`.
    /// The other ends of the streams are then used to create a [`Handle`].
    /// This function is only a helper to convert any [`wasmtime::Error`] into a [`std::io::Error`] if necessary before returning.
    /// Returns the [`Handle`] and a [`Sender`] that can be used to send [`ProcessCommand`]s to the process.
    pub(super) fn launch(&mut self, path: &Path, args: &[String], event_sender: &Sender<ProcessEvent>, id: usize) -> Result<(Sender<ProcessCommand>, Handle), Error> {
        let (stdin, stdout, stderr, start_fn) = match self.do_launch(path, args) {
            Ok(ret) => ret,
            Err(e) => return Err(into_io_error(e)),
        };
        Handle::new(id, path, event_sender, stdin, stdout, stderr, start_fn)
    }

    /// Helper function to actually launch a Webassembly process. See ['WasmLauncher::launch'].
    fn do_launch(&mut self, path: &Path, args: &[String]) -> Result<(MemoryWriter, MemoryReader, MemoryReader, impl FnOnce() -> i32), wasmtime::Error> {
        log::info!("launching wasm file {}, args: {args:?}", path.display());
        let module = self.load_module(path)?;
        let (stdin, wasi_stdin) = in_memory_pipe();
        let (wasi_stdout, stdout) = in_memory_pipe();
        let (wasi_stderr, stderr) = in_memory_pipe();

        let wasi_ctx = WasiCtxBuilder::new()
            .args(args).unwrap()
            .stdin(Box::new(wasi_stdin))
            .stdout(Box::new(wasi_stdout))
            .stderr(Box::new(wasi_stderr))
            .build();

        let mut store = Store::new(&self.engine, wasi_ctx);

        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker(&mut linker, |cx| cx)?;

        linker.module(&mut store, "", &module)?;

        let start_fn = linker.get_default(&mut store, "")?
            .typed::<(), ()>(&mut store)?;
        Ok((stdin, stdout, stderr, move || {
            match start_fn.call(store, ()) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        }))
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

/// Creates a new [`MemoryStream`] and returns a [`MemoryReader`] and [`MemoryWriter`] sharing that stream
fn in_memory_pipe() -> (MemoryWriter, MemoryReader) {
    let inner = Arc::new(MemoryStream {
        data: Mutex::new(VecDeque::new()),
        data_available: Condvar::new(),
        writer_closed: AtomicBool::new(false),
    });
    (MemoryWriter { inner: inner.clone() }, MemoryReader { inner })
}

/// Converts a [`wasmtime::Error`] into a [`std::io::Error`].
fn into_io_error(error: wasmtime::Error) -> Error {
    for e in error.chain() {
        if let Some(e) = e.downcast_ref::<Error>() {
            return Error::new(e.kind(), e.to_string());
        }
    }
    return Error::new(std::io::ErrorKind::Other, error.to_string());
}

impl MemoryReader {
    /// helper function that waits until data is available to read or the corresponding [`MemoryWriter`] was dropped.
    fn wait_for_data(&self) -> Option<MutexGuard<VecDeque<u8>>> {
        let mut data = self.inner.data.lock().unwrap();
        while data.len() == 0 {
            data = self.inner.data_available.wait(data).unwrap();
            if self.inner.writer_closed.load(Ordering::Acquire) { return None; }
        }
        Some(data)
    }
}

impl Read for MemoryReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(mut data) = self.wait_for_data() {
            let num_bytes = data.read(buf)
                .expect("VecDeque::read never fails");
            Ok(num_bytes)
        } else { Ok(0) }
    }
}

impl Write for MemoryWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let num_bytes = self.inner.data.lock().unwrap().write(buf)
            .expect("VecDeque::write never fails");
        if num_bytes > 0 {
            self.inner.data_available.notify_all();
        }
        Ok(num_bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[wiggle::async_trait]
impl WasiFile for MemoryReader {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn get_filetype(&self) -> Result<FileType, wasi_common::Error> {
        Ok(FileType::Unknown)
    }

    async fn read_vectored<'a>(&self, bufs: &mut [IoSliceMut<'a>]) -> Result<u64, wasi_common::Error> {
        if let Some(mut data) = self.wait_for_data() {
            let num_bytes = data.read_vectored(bufs)
                .expect("VecDeque::read_vectored never fails");
            Ok(num_bytes as u64)
        } else { Ok(0) }
    }

    async fn readable(&self) -> Result<(), wasi_common::Error> {
        Ok(())
    }
}

#[wiggle::async_trait]
impl WasiFile for MemoryWriter {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn get_filetype(&self) -> Result<FileType, wasi_common::Error> {
        Ok(FileType::Unknown)
    }

    async fn write_vectored<'a>(&self, bufs: &[IoSlice<'a>]) -> Result<u64, wasi_common::Error> {
        let num_bytes = self.inner.data.lock().unwrap().write_vectored(bufs)
            .expect("VecDeque::write_vectored never fails");
        if num_bytes > 0 {
            self.inner.data_available.notify_all();
        }
        Ok(num_bytes as u64)
    }

    async fn writable(&self) -> Result<(), wasi_common::Error> {
        Ok(())
    }
}

impl Drop for MemoryWriter {
    fn drop(&mut self) {
        self.inner.writer_closed.store(true, Ordering::Release);
        self.inner.data_available.notify_all();
    }
}