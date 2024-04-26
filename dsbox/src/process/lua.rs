//! lua scripts can be used as an implementation of a node
//! todo: more documentation

use std::io::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

use mlua::{Lua, LuaOptions, LuaSerdeExt, MultiValue, StdLib, Table, Value};
use tokio::sync::mpsc::{Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::Mutex;
use tokio::sync::oneshot::Receiver;

use crate::process::command::ProcessCommand;
use crate::process::event::{ProcessEvent, ProcessEventKind};
use crate::process::handle::Handle;

/// gets passed to the lua instance via [`Lua::set_app_data`] and is then available in the
/// native function implementations
struct LuaAppData {
    /// the id of the "process", used when sending events to the core
    id: usize,
    /// a sender to send events to the core
    sender: Sender<ProcessEvent>,
    /// a receiver to receive commands (currently only messages) from the core
    receiver: Mutex<UnboundedReceiver<ProcessCommand>>,
}

/// launches a new lua script. the lua script has access to the passed arguments via a global `args` table.
pub(super) fn launch(file: &Path, args: Vec<String>, event_sender: &Sender<ProcessEvent>, id: usize) -> Result<(UnboundedSender<ProcessCommand>, Handle), Error> {
    log::trace!("launching lua node `{}`, args: {args:?}", file.display());
    let (command_sender, command_receiver) = tokio::sync::mpsc::unbounded_channel();
    let app_data = LuaAppData {
        id,
        sender: event_sender.clone(),
        receiver: Mutex::new(command_receiver),
    };

    let lua_thread = launch_lua(file.to_path_buf(), args, app_data);

    let finish_handle = {
        let file = file.to_path_buf();
        async move {
            let result = lua_thread.await.unwrap();
            if let Err(e) = &result {
                log::warn!("script `{}` exited with an error: {e}", file.display());
            }
            result.unwrap_or_else(|_| -1)
        }
    };

    Ok((command_sender, Handle::for_lua(id, event_sender.clone(), finish_handle)))
}

// runs the given lua script a separate thread, because we cannot pre-emptively interrupt them
fn launch_lua(file: PathBuf, args: Vec<String>, app_data: LuaAppData) -> Receiver<mlua::Result<i32>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let rt = tokio::runtime::Handle::current();
    std::thread::Builder::new()
        .name(format!("{}", file.display()))
        .spawn(move || {
            let lua = setup_lua(&args, app_data)
                .expect("failed to setup lua");
            let result = rt.block_on(run_lua(lua, file));
            tx.send(result).unwrap();
        }).unwrap();
    rx
}

async fn run_lua(lua: Lua, file: PathBuf) -> mlua::Result<i32> {
    let chunk = lua.load(file);
    let result: Value = chunk.call_async(()).await?;
    Ok(result.as_i32().unwrap_or(0))
}

/// creates a new [`Lua`] instance and sets it up with some globals, like `args`, send and receive
/// functions, a Message class etc.
fn setup_lua(args: &[String], app_data: LuaAppData) -> mlua::Result<Lua> {
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::PACKAGE,
        LuaOptions::new(),
    )?;
    lua.set_app_data(app_data);
    let args_table = lua.create_table()?;
    for arg in args {
        args_table.push(arg.as_str())?;
    }
    lua.globals().set("args", args_table)?;
    lua.globals().set("send", lua.create_async_function(LuaAppData::lua_send)?)?;
    lua.globals().set("recv", lua.create_async_function(LuaAppData::lua_recv)?)?;
    lua.globals().set("recv_iter", lua.create_function(LuaAppData::lua_recv_iter)?)?;
    lua.globals().set("log", lua.create_async_function(LuaAppData::lua_log)?)?;
    lua.globals().set("sleep", lua.create_async_function(sleep)?)?;
    let message_class = lua.create_table()?;
    message_class.set("new", lua.create_function(message_new)?)?;
    message_class.set("reply", lua.create_function(message_reply)?)?;
    message_class.set("send", lua.create_async_function(message_send)?)?;
    lua.globals().set("Message", message_class)?;

    Ok(lua)
}

impl LuaAppData {
    /// attempts to deserialize the given Value as a [`libproto::Message`] using `mlua`s `serde` support
    /// and sends it to the core
    async fn lua_send(lua: &Lua, message: Value<'_>) -> mlua::Result<bool> {
        let app_data = lua.app_data_ref::<Self>().unwrap();
        let message = match lua.from_value(message.clone()) {
            Ok(message) => message,
            Err(e) => {
                let raw_message = serde_json::to_string(&message)
                    .unwrap();
                app_data.send_event(ProcessEventKind::SerializeError { raw_message, error: e.to_string() }).await.ok();
                return Ok(false);
            }
        };
        Ok(app_data.send_event(ProcessEventKind::Message(message)).await.is_ok())
    }

    /// waits for a single message to be received by the core with an optional timeout given in seconds
    /// if no message is available `nil` is returned.
    async fn lua_recv<'lua>(lua: &'lua Lua, params: (Option<f64>, MultiValue<'lua>)) -> mlua::Result<Option<Value<'lua>>> {
        let (timeout, _rest) = params;
        let app_data = lua.app_data_ref::<Self>().unwrap();
        let Some(message) = app_data.recv_command(timeout.map(Duration::from_secs_f64)).await else { return Ok(None); };
        match message {
            ProcessCommand::Deliver(message) => {
                let value = lua.to_value(&message)?;
                if let Some(value) = value.as_table() {
                    let index = lua.create_table()?;
                    index.set("__index", lua.globals().get::<&str, Value>("Message")?)?;
                    value.set_metatable(Some(index));
                }
                Ok(Some(value))
            }
        }
    }

    /// waits for a single message to be received by the core with an optional timeout given in seconds
    /// if no message is available `nil` is returned.
    fn lua_recv_iter(lua: &Lua, _: ()) -> mlua::Result<Option<Value>> {
        return lua.globals().get("recv");
    }

    async fn lua_log(lua: &Lua, items: MultiValue<'_>) -> mlua::Result<bool> {
        let app_data = lua.app_data_ref::<Self>().unwrap();
        let mut message = String::new();
        let mut first = false;
        for item in items {
            if first { first = false; } else { message.push(' '); }
            message.push_str(&Self::serialize(item))
        }
        Ok(app_data.send_event(ProcessEventKind::Log(message)).await.is_ok())
    }

    fn serialize(value: Value<'_>) -> String {
        match value {
            Value::Nil => "nil".to_string(),
            Value::Boolean(v) => v.to_string(),
            Value::LightUserData(v) => format!("userdata: {:#x}", v.0 as usize),
            Value::Integer(v) => v.to_string(),
            Value::Number(v) => v.to_string(),
            Value::String(v) => v.to_string_lossy().to_string(),
            Value::Table(v) => {
                serde_json::to_string(&v)
                    .unwrap_or_else(|_| format!("table: {:#x}", v.to_pointer() as usize))
            }
            Value::Function(v) => format!("function: {:#x}", v.to_pointer() as usize),
            Value::Thread(v) => format!("thread: {:#x}", v.to_pointer() as usize),
            Value::UserData(v) => format!("userdata: {:#x}", v.to_pointer() as usize),
            Value::Error(e) => e.to_string(),
        }
    }

    async fn send_event(&self, kind: ProcessEventKind) -> Result<(), SendError<ProcessEvent>> {
        self.sender.send(ProcessEvent::new(self.id, kind)).await
    }

    async fn recv_command(&self, timeout: Option<Duration>) -> Option<ProcessCommand> {
        let mut receiver = self.receiver.lock().await;
        if let Some(timeout) = timeout {
            tokio::time::timeout(timeout, receiver.recv())
                .await.ok().flatten()
        } else {
            receiver.recv().await
        }
    }
}

fn message_new<'lua>(lua: &'lua Lua, params: (Table<'lua>, Value<'lua>, Value<'lua>, Value<'lua>, MultiValue<'lua>)) -> mlua::Result<Table<'lua>> {
    let (message_class, src, dst, r#type, rest) = params;
    let new_message = lua.create_table()?;
    let index = lua.create_table()?;
    index.set("__index", message_class)?;
    new_message.set_metatable(Some(index));
    new_message.set("src", src)?;
    new_message.set("dest", dst)?;
    let body = lua.create_table()?;
    body.set("type", r#type)?;
    merge_into_table(&body, rest)?;
    new_message.set("body", body)?;
    Ok(new_message)
}

fn message_reply<'lua>(lua: &'lua Lua, params: (Table<'lua>, Value<'lua>, MultiValue<'lua>)) -> mlua::Result<Table<'lua>> {
    let (message, r#type, rest) = params;
    let message_class = if let Some(metatable) = message.get_metatable() {
        metatable.get("__index")?
    } else { lua.create_table()? };
    let new_message = message_new(lua, (message_class, message.get("dest")?, message.get("src")?, r#type, MultiValue::new()))?;
    let body: Table = new_message.get("body")?;
    body.set("in_reply_to", message.get::<&str, Value>("id")?)?;
    merge_into_table(&body, rest)?;
    Ok(new_message)
}

async fn message_send<'lua>(lua: &'lua Lua, params: (Value<'lua>, MultiValue<'lua>)) -> mlua::Result<bool> {
    let (message, _rest) = params;
    LuaAppData::lua_send(lua, message).await
}

fn merge_into_table(table: &Table, multi: MultiValue) -> mlua::Result<()> {
    for v in multi {
        let Some(t) = v.as_table() else { continue; };
        t.for_each(|k: Value, v: Value| {
            table.set(k, v)
        })?;
    }
    Ok(())
}

async fn sleep<'lua>(_: &'lua Lua, params: (f64, MultiValue<'lua>)) -> mlua::Result<()> {
    let (secs, _rest) = params;
    tokio::time::sleep(Duration::from_secs_f64(secs)).await;
    Ok(())
}