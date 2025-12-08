//! Transparent handling of processes, both native, compiled to Webassembly and in the form of a lua script.

pub use command::ProcessCommand;
pub use event::ProcessEvent;
pub(crate) use event::ProcessEventOrExit;
pub(crate) use runner::handle::RunningHandle;
pub(crate) use runner::manager::RunnerManger;
pub use runner::{callback::CallbackRunner, callback::CallbackOnceRunner, native::NativeRunner, CommandReceiver, EventSender};

#[cfg(feature = "lua")]
pub use runner::lua::LuaRunner;

#[cfg(feature = "wasm")]
pub use runner::wasm::WasmRunner;

pub use runner::Runner;

mod command;
mod event;
mod runner;
