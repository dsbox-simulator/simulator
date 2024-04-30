//! lua scripts can be used as an implementation of a node
//! todo: more documentation

use std::path::{Path, PathBuf};
use std::time::Duration;

use mlua::{FromLua, FromLuaMulti, Function, IntoLua, Lua, LuaOptions, LuaSerdeExt, MultiValue, StdLib, Table, Value};
use tokio::sync::{Mutex, oneshot};
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::sync::mpsc::error::SendError;
use tokio::task::JoinHandle;

use libproto::Message;

use crate::process::command::ProcessCommand;
use crate::process::event::ProcessEvent;

/// gets passed to the lua instance via [`Lua::set_app_data`] and is then available in the
/// native function implementations
struct LuaAppData {
    /// a sender to send events to the core
    sender: Sender<ProcessEvent>,
    /// a receiver to receive commands (currently only messages) from the core
    receiver: Mutex<UnboundedReceiver<ProcessCommand>>,
}

/// launches a new lua script. the lua script has access to the passed arguments via a global `args` table.
pub(super) fn launch(file: &Path, args: Vec<String>, allow_os_libs: bool, command_receiver: UnboundedReceiver<ProcessCommand>, event_sender: Sender<ProcessEvent>) -> tokio::io::Result<(JoinHandle<()>, oneshot::Receiver<()>)> {
    log::trace!("launching lua node `{}`, args: {args:?}", file.display());
    let app_data = LuaAppData {
        sender: event_sender,
        receiver: Mutex::new(command_receiver),
    };

    let (finished_tx, finished_rx) = oneshot::channel();
    let lua_thread = launch_lua(file.to_path_buf(), args, allow_os_libs, app_data, finished_tx);
    Ok((lua_thread, finished_rx))
}

// runs the given lua script a separate thread, because we cannot pre-emptively interrupt them
fn launch_lua(file: PathBuf, args: Vec<String>, allow_os_libs: bool, app_data: LuaAppData, finished: oneshot::Sender<()>) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Handle::current();
        let lua = setup_lua(&file, &args, allow_os_libs, app_data)
            .expect("failed to setup lua");
        rt.block_on(run_lua(lua, &file));
        finished.send(()).ok();
    })
}

async fn run_lua(lua: Lua, file: &Path) {
    let chunk = lua.load(file);
    let result = chunk.call_async(()).await
        .map(|v: Value| v.as_i32().unwrap_or(0));
    if let Err(e) = &result {
        log::warn!("script `{}` exited with an error: {e}", file.display());
    }
    let exit_code = result.as_ref().ok().copied().unwrap_or(-1);
    let app_data = lua.app_data_ref::<LuaAppData>().unwrap();
    app_data.sender.send(ProcessEvent::Exited(exit_code)).await.ok();
}

/// creates a new [`Lua`] instance and sets it up with some globals, like `args`, send and receive
/// functions, a Message class etc.
fn setup_lua(lua_file: &Path, args: &[String], allow_os_libs: bool, app_data: LuaAppData) -> mlua::Result<Lua> {
    let libs = StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::COROUTINE | StdLib::MATH | StdLib::PACKAGE;
    let lua = if allow_os_libs {
        unsafe {
            Lua::unsafe_new_with(libs | StdLib::OS | StdLib::IO, LuaOptions::new())
        }
    } else { Lua::new_with(libs, LuaOptions::new())? };
    if allow_os_libs {}
    lua.set_app_data(app_data);
    let args_table = lua.create_table()?;
    for arg in args {
        args_table.push(arg.as_str())?;
    }
    let mod_dsbox = lua.create_table()?;

    mod_dsbox.set("args", args_table)?;
    mod_dsbox.set("send", lua.create_async_function(LuaAppData::lua_send)?)?;
    mod_dsbox.set("recv", lua.create_async_function(LuaAppData::lua_recv)?)?;
    mod_dsbox.set("recv_iter", lua.create_function(LuaAppData::lua_recv_iter)?)?;
    mod_dsbox.set("sleep", lua.create_async_function(sleep)?)?;
    mod_dsbox.set("array", lua.create_function(lua_array)?)?;
    let message_class = lua.create_table()?;
    message_class.set("new", lua.create_function(message_new)?)?;
    message_class.set("create_reply", lua.create_function(message_create_reply)?)?;
    message_class.set("reply", lua.create_async_function(message_reply)?)?;
    message_class.set("send", lua.create_async_function(message_send)?)?;
    message_class.set("tostring", lua.create_function(message_to_string)?)?;
    mod_dsbox.set("Message", message_class)?;

    lua.globals().set("print", lua.create_async_function(LuaAppData::lua_print)?)?;

    // for some reason the borrow checker does not like having the local variables `package` and `preload`
    // hanging around, so we wrap the following code in a block to make them drop explicitly before returning
    {
        let package: Table = lua.globals().get("package")?;
        let loaded: Table = package.get("loaded")?;
        loaded.set("dsbox", mod_dsbox)?;

        macro_rules! join_path {
            ($($p:expr),*) => {{
                let mut path = PathBuf::new();
                $(path.push($p);)*
                path
            }};
        }

        // setup luarocks support
        let version: String = lua.globals().get("_VERSION")?;
        let source_path = lua_file.parent().unwrap();
        let local_path = join_path!(source_path, "lua_modules", "share", "lua", &version[4..]);
        let search1 = join_path!(&local_path, "?.lua");
        let search2 = join_path!(&local_path, "?", "init.lua");

        // let home_path = join_path!("~")

        let path: String = package.get("path")?;
        let full_path = format!("{};{};{path}", search1.display(), search2.display());
        package.set("path", full_path)?;
    }

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
                app_data.send_event(ProcessEvent::SerializeError { raw_message, error: e.to_string() }).await.ok();
                return Ok(false);
            }
        };
        Ok(app_data.send_event(ProcessEvent::Message(message)).await.is_ok())
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
                if let Some(message) = value.as_table() {
                    let message_class: Table = get_dsbox(lua, "Message")?;
                    message_set_metatable(lua, message, &message_class)?;
                }
                Ok(Some(value))
            }
        }
    }

    /// waits for a single message to be received by the core with an optional timeout given in seconds
    /// if no message is available `nil` is returned.
    fn lua_recv_iter(lua: &Lua, _: ()) -> mlua::Result<Value> {
        get_dsbox(lua, "recv")
    }

    async fn lua_print(lua: &Lua, items: MultiValue<'_>) -> mlua::Result<bool> {
        let app_data = lua.app_data_ref::<Self>().unwrap();
        let mut message = String::new();
        let mut first = true;
        for item in items {
            if first { first = false; } else { message.push('\t'); }
            message.push_str(&lua.globals().get::<&str, Function>("tostring")?.call::<Value, String>(item)?);
        }
        Ok(app_data.send_event(ProcessEvent::Log(message)).await.is_ok())
    }

    async fn send_event(&self, event: ProcessEvent) -> Result<(), SendError<ProcessEvent>> {
        self.sender.send(event).await
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
    new_message.set("src", src)?;
    new_message.set("dest", dst)?;
    let body = lua.create_table()?;
    body.set("type", r#type)?;
    merge_into_table(&body, rest)?;
    new_message.set("body", body)?;
    message_set_metatable(lua, &new_message, &message_class)?;
    Ok(new_message)
}

fn message_set_metatable(lua: &Lua, message: &Table, message_class: &Table) -> mlua::Result<()> {
    let metatable = lua.create_table()?;
    let tostring: Function = message_class.get("tostring")?;
    metatable.set("__tostring", tostring)?;
    metatable.set("__index", message_class)?;
    message.set_metatable(Some(metatable));
    Ok(())
}

fn message_create_reply<'lua>(lua: &'lua Lua, params: (Table<'lua>, Value<'lua>, MultiValue<'lua>)) -> mlua::Result<Table<'lua>> {
    let (message, r#type, rest) = params;
    let message_class = if let Some(metatable) = message.get_metatable() {
        metatable.get("__index")?
    } else { lua.create_table()? };
    let body: Table = message.get("body")?;
    let new_message = message_new(lua, (message_class, message.get("dest")?, message.get("src")?, r#type, MultiValue::new()))?;
    let new_body: Table = new_message.get("body")?;
    new_body.set("in_reply_to", body.get::<&str, Value>("msg_id")?)?;
    merge_into_table(&new_body, rest)?;
    Ok(new_message)
}

async fn message_reply<'lua>(lua: &'lua Lua, params: (Table<'lua>, Value<'lua>, MultiValue<'lua>)) -> mlua::Result<bool> {
    let message = message_create_reply(lua, params)?;
    message_send(lua, (message, MultiValue::new())).await
}

async fn message_send<'lua>(lua: &'lua Lua, params: (Table<'lua>, MultiValue<'lua>)) -> mlua::Result<bool> {
    let (mut message, mut rest) = params;
    let message_class: Table = get_dsbox(lua, "Message")?;
    if message == message_class {
        rest.push_front(message_class.into_lua(lua)?);
        message = message_new(lua, FromLuaMulti::from_lua_multi(rest, lua)?)?;
    }
    LuaAppData::lua_send(lua, message.into_lua(lua)?).await
}

fn message_to_string(lua: &Lua, message: Value) -> mlua::Result<String> {
    let message: Message = lua.from_value(message)?;
    Ok(message.to_json())
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

fn lua_array<'lua>(lua: &'lua Lua, table: Table<'lua>) -> mlua::Result<Table<'lua>> {
    table.set_metatable(Some(lua.array_metatable()));
    Ok(table)
}

fn get_dsbox<'lua, K: IntoLua<'lua>, V: FromLua<'lua>>(lua: &'lua Lua, key: K) -> mlua::Result<V> {
    lua.globals().get::<&str, Table>("package")?
        .get::<&str, Table>("loaded")?
        .get::<&str, Table>("dsbox")?
        .get(key)
}