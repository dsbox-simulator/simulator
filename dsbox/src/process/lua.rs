//! lua scripts can be used as an implementation of a node
//! todo: more documentation

use std::path::{Path, PathBuf};
use std::process::Stdio;

use mlua::{FromLua, FromLuaMulti, Function, IntoLua, Lua, LuaOptions, LuaSerdeExt, MultiValue, StdLib, Table, Value};
use tokio::sync::{Mutex, oneshot};
use tokio::sync::mpsc::{Sender, UnboundedReceiver};
use tokio::sync::mpsc::error::SendError;
use tokio::task::JoinHandle;

use libproto::Message;
use libproto::services::{LogMarker, LogMarkerColor, LogMessage};

use crate::core::CORE_NAME;
use crate::process::command::ProcessCommand;
use crate::process::event::ProcessEvent;

pub struct LuaLauncher {
    luarocks_path: Option<String>,
    luarocks_cpath: Option<String>,
}

impl LuaLauncher {
    pub async fn new() -> Self {
        let (path, cpath) = if let Ok((path, cpath)) = Self::query_luarocks_path().await {
            (Some(path), Some(cpath))
        } else { (None, None) };
        Self {
            luarocks_path: path,
            luarocks_cpath: cpath,
        }
    }

    async fn query_luarocks_path() -> tokio::io::Result<(String, String)> {
        let path = tokio::process::Command::new("luarocks")
            .args(["path", "--lr-path"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output().await?;
        let cpath = tokio::process::Command::new("luarocks")
            .args(["path", "--lr-cpath"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output().await?;
        let path = String::from_utf8_lossy(&path.stdout).to_string();
        let cpath = String::from_utf8_lossy(&cpath.stdout).to_string();
        Ok((path, cpath))
    }

    /// launches a new lua script. the lua script has access to the passed arguments via a global `args` table.
    pub fn launch(&self, file: &Path, args: Vec<String>, allow_os_libs: bool, command_receiver: UnboundedReceiver<ProcessCommand>, event_sender: Sender<ProcessEvent>, name: String) -> tokio::io::Result<(JoinHandle<()>, oneshot::Receiver<()>)> {
        log::trace!("launching lua node `{}`, args: {args:?}", file.display());
        let app_data = LuaAppData {
            sender: event_sender,
            receiver: Mutex::new(command_receiver),
            name,
        };

        let (finished_tx, finished_rx) = oneshot::channel();
        let lua_thread = self.launch_lua(file.to_path_buf(), args, allow_os_libs, app_data, finished_tx);
        Ok((lua_thread, finished_rx))
    }


    // runs the given lua script a separate thread, because we cannot pre-emptively interrupt them
    fn launch_lua(&self, file: PathBuf, args: Vec<String>, allow_os_libs: bool, app_data: LuaAppData, finished: oneshot::Sender<()>) -> JoinHandle<()> {
        let path = self.luarocks_path.clone();
        let cpath = self.luarocks_cpath.clone();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            let lua = Self::setup_lua(&file, &args, allow_os_libs, path, cpath, app_data)
                .expect("failed to setup lua");
            rt.block_on(Self::run_lua(lua, &file));
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
    fn setup_lua(lua_file: &Path, args: &[String], allow_os_libs: bool, path: Option<String>, cpath: Option<String>, app_data: LuaAppData) -> mlua::Result<Lua> {
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
        mod_dsbox.set("array", lua.create_function(lua_array)?)?;
        mod_dsbox.set("to_json", lua.create_function(lua_to_json)?)?;
        mod_dsbox.set("log", lua.create_async_function(LuaAppData::lua_log)?)?;
        let message_class = lua.create_table()?;
        message_class.set("new", lua.create_function(message_new)?)?;
        message_class.set("create_reply", lua.create_function(message_create_reply)?)?;
        message_class.set("reply", lua.create_async_function(message_reply)?)?;
        message_class.set("send", lua.create_async_function(message_send)?)?;
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

            // setup module search paths
            let version: String = lua.globals().get("_VERSION")?;
            let mut path = if let Some(path) = path { path } else { package.get("path")? };
            let mut cpath = if let Some(cpath) = cpath { cpath } else { package.get("cpath")? };
            let mut push_path = |next_path: PathBuf| {
                path.push(';');
                path.push_str(next_path.to_string_lossy().as_ref())
            };

            let source_path = lua_file.parent().unwrap();
            // search for modules in the current scripts directory
            // search for a file named `<modname>.lua`
            push_path(join_path!(source_path, "?.lua"));
            // or search for a folder named `<modname>` with a file called `init.lua`
            push_path(join_path!(source_path, "?", "init.lua"));

            // search for rocks installed in the current scripts directory, in a subfolder called `lua_modules`
            // search in `lua_modules/share/lua/<lua version>/
            let local_path = join_path!(source_path, "lua_modules", "share", "lua", &version[4..]);
            // search there for a file named `<modname>.lua`
            push_path(join_path!(&local_path, "?.lua"));
            // or search there for a folder named `<modname>` with a file called `init.lua`
            push_path(join_path!(&local_path, "?", "init.lua"));

            let mut push_cpath = |next_path: PathBuf| {
                cpath.push(';');
                cpath.push_str(next_path.to_string_lossy().as_ref())
            };
            // search for C modules in the current scripts directory
            // search for a file named `<modname>.so`
            push_cpath(join_path!(source_path, "?.so"));

            // search for rocks installed in the current scripts directory, in a subfolder called `lua_modules`
            // search in `lua_modules/lib64/lua/<lua version>/<modname>.so
            push_cpath(join_path!(source_path, "lua_modules", "lib64", "lua", &version[4..], "?.so"));

            package.set("path", path)?;
            package.set("cpath", cpath)?;
        }

        Ok(lua)
    }
}

/// gets passed to the lua instance via [`Lua::set_app_data`] and is then available in the
/// native function implementations
struct LuaAppData {
    /// a sender to send events to the core
    sender: Sender<ProcessEvent>,
    /// a receiver to receive commands (currently only messages) from the core
    receiver: Mutex<UnboundedReceiver<ProcessCommand>>,
    /// the name of this node (useful for automatically sending log messages with extended information to the core)
    name: String,
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

    /// waits for a single message to be received by the core
    /// If no more messages are available `nil` is returned.
    async fn lua_recv<'lua>(lua: &'lua Lua, _: ()) -> mlua::Result<Option<Value<'lua>>> {
        let app_data = lua.app_data_ref::<Self>().unwrap();
        let Some(command) = app_data.recv_command().await else { return Ok(None); };
        match command {
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

    /// returns an iterator that iterates over all received messages until there are no more
    /// messages to be received (i.e. when the simulation shuts down)
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

    async fn lua_log(lua: &Lua, (message, label, color, _rest): (String, Option<String>, Option<String>, MultiValue<'_>)) -> mlua::Result<bool> {
        let app_data = lua.app_data_ref::<Self>().unwrap();
        let marker = if let Some(label) = label {
            let color = if let Some(color) = color {
                log_marker_color_from_str(&color)
            } else { None };
            Some(LogMarker { label, color })
        } else { None };
        Ok(app_data.send_event(ProcessEvent::Message(Message::new(&app_data.name, CORE_NAME, None, LogMessage {
            text: message,
            marker,
        }))).await.is_ok())
    }

    async fn send_event(&self, event: ProcessEvent) -> Result<(), SendError<ProcessEvent>> {
        self.sender.send(event).await
    }

    async fn recv_command(&self) -> Option<ProcessCommand> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }
}

fn message_new<'lua>(lua: &'lua Lua, (message_class, src, dst, r#type, rest): (Table<'lua>, Value<'lua>, Value<'lua>, Value<'lua>, MultiValue<'lua>)) -> mlua::Result<Table<'lua>> {
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
    let tostring: Function = get_dsbox(lua, "to_json")?;
    metatable.set("__tostring", tostring)?;
    metatable.set("__index", message_class)?;
    message.set_metatable(Some(metatable));
    Ok(())
}

fn message_create_reply<'lua>(lua: &'lua Lua, (message, r#type, rest): (Table<'lua>, Value<'lua>, MultiValue<'lua>)) -> mlua::Result<Table<'lua>> {
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

async fn message_reply<'lua>(lua: &'lua Lua, (message, r#type, rest): (Table<'lua>, Value<'lua>, MultiValue<'lua>)) -> mlua::Result<bool> {
    let message = message_create_reply(lua, (message, r#type, rest))?;
    message_send(lua, (message, MultiValue::new())).await
}

async fn message_send<'lua>(lua: &'lua Lua, (mut message, mut rest): (Table<'lua>, MultiValue<'lua>)) -> mlua::Result<bool> {
    let message_class: Table = get_dsbox(lua, "Message")?;
    if message == message_class {
        rest.push_front(message_class.into_lua(lua)?);
        message = message_new(lua, FromLuaMulti::from_lua_multi(rest, lua)?)?;
    }
    LuaAppData::lua_send(lua, message.into_lua(lua)?).await
}

fn lua_to_json(lua: &Lua, value: Value) -> mlua::Result<String> {
    let value: serde_json::Value = lua.from_value(value)?;
    Ok(serde_json::to_string(&value).unwrap())
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

pub fn log_marker_color_from_str(color: &str) -> Option<LogMarkerColor> {
    match color {
        "black" => Some(LogMarkerColor::Black),
        "red" => Some(LogMarkerColor::Red),
        "green" => Some(LogMarkerColor::Green),
        "yellow" => Some(LogMarkerColor::Yellow),
        "blue" => Some(LogMarkerColor::Blue),
        "magenta" => Some(LogMarkerColor::Magenta),
        "cyan" => Some(LogMarkerColor::Cyan),
        "white" => Some(LogMarkerColor::White),
        "bright_black" => Some(LogMarkerColor::BrightBlack),
        "bright_red" => Some(LogMarkerColor::BrightRed),
        "bright_green" => Some(LogMarkerColor::BrightGreen),
        "bright_yellow" => Some(LogMarkerColor::BrightYellow),
        "bright_blue" => Some(LogMarkerColor::BrightBlue),
        "bright_magenta" => Some(LogMarkerColor::BrightMagenta),
        "bright_cyan" => Some(LogMarkerColor::BrightCyan),
        "bright_white" => Some(LogMarkerColor::BrightWhite),
        _ => None
    }
}