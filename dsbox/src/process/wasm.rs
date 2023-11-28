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

use crate::process::handle::Handle;
use crate::process::{ProcessCommand, ProcessEvent};

pub struct WasmLauncher {
    engine: Engine,
    module_cache: HashMap<PathBuf, Module>,
}

struct MemoryReader {
    inner: Arc<MemoryStream>,
}

struct MemoryWriter {
    inner: Arc<MemoryStream>,
}

struct MemoryStream {
    data: Mutex<VecDeque<u8>>,
    data_available: Condvar,
    writer_closed: AtomicBool,
}

impl WasmLauncher {
    pub fn new() -> Self {
        let config = Config::new();
        Self { engine: Engine::new(&config).unwrap(), module_cache: HashMap::new() }
    }

    pub(super) fn spawn(&mut self, file: &Path, event_sender: &Sender<ProcessEvent>, id: usize) -> Result<(Sender<ProcessCommand>, Handle), Error> {
        let (stdin, stdout, stderr, start_fn) = match self.do_spawn(file) {
            Ok(ret) => ret,
            Err(e) => return Err(into_io_error(e)),
        };
        Handle::new(id, file, event_sender, stdin, stdout, stderr, start_fn)
    }

    fn do_spawn(&mut self, file: &Path) -> Result<(MemoryWriter, MemoryReader, MemoryReader, impl FnOnce() -> i32), wasmtime::Error> {
        let module = self.load_module(file)?;
        let (stdin, wasi_stdin) = in_memory_pipe();
        let (wasi_stdout, stdout) = in_memory_pipe();
        let (wasi_stderr, stderr) = in_memory_pipe();

        let wasi_ctx = WasiCtxBuilder::new()
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

    fn load_module(&mut self, file: &Path) -> Result<Module, wasmtime::Error> {
        if let Some(module) = self.module_cache.get(file) {
            Ok(module.clone())
        } else {
            let module = Module::from_file(&self.engine, file)?;
            self.module_cache.insert(file.to_path_buf(), module.clone());
            Ok(module)
        }
    }
}

fn in_memory_pipe() -> (MemoryWriter, MemoryReader) {
    let inner = Arc::new(MemoryStream {
        data: Mutex::new(VecDeque::new()),
        data_available: Condvar::new(),
        writer_closed: AtomicBool::new(false),
    });
    (MemoryWriter { inner: inner.clone() }, MemoryReader { inner })
}

fn into_io_error(error: wasmtime::Error) -> Error {
    for e in error.chain() {
        if let Some(e) = e.downcast_ref::<Error>() {
            return Error::new(e.kind(), e.to_string());
        }
    }
    return Error::new(std::io::ErrorKind::Other, error.to_string());
}

impl MemoryReader {
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