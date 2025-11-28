use super::{Abort, Exit};

use crate::process::runner::lua::appdata::DsboxData;
use crate::process::{ProcessCommand, ProcessEvent};
use libproto::services::{LogMarker, LogMarkerColor, LogMessage};
use libproto::Message;
use mlua::{
    Error, FromLua, FromLuaMulti, Function, IntoLua, Lua, LuaSerdeExt, MultiValue, Number, Result,
    Table, Value,
};
use std::sync::Arc;
use std::time::Duration;

pub fn init_dsbox_module(lua: &Lua, args: &[String]) -> Result<()> {
    let mod_dsbox = lua.create_table()?;

    let args_table = lua.create_table()?;
    for arg in args {
        args_table.push(arg.as_str())?;
    }
    mod_dsbox.set("args", args_table)?;

    mod_dsbox.set("send", lua.create_function(lua_send)?)?;
    mod_dsbox.set("recv", lua.create_function(lua_recv)?)?;
    mod_dsbox.set("recv_iter", lua.create_function(lua_recv_iter)?)?;
    mod_dsbox.set("array", lua.create_function(lua_array)?)?;
    mod_dsbox.set("to_json", lua.create_function(lua_to_json)?)?;
    mod_dsbox.set("log", lua.create_function(lua_log)?)?;
    mod_dsbox.set("clock", lua.create_function(lua_clock)?)?;
    mod_dsbox.set("sleep", lua.create_function(lua_sleep)?)?;
    mod_dsbox.set("exit", lua.create_function(lua_exit)?)?;
    let message_class = lua.create_table()?;
    message_class.set("new", lua.create_function(message_new)?)?;
    message_class.set("create_reply", lua.create_function(message_create_reply)?)?;
    message_class.set("reply", lua.create_function(message_reply)?)?;
    message_class.set("send", lua.create_function(message_send)?)?;
    mod_dsbox.set("Message", message_class)?;

    lua.globals()
        .set("print", lua.create_function(lua_print)?)?;

    lua.register_module("dsbox", mod_dsbox)?;

    Ok(())
}

/// attempts to deserialize the given Value as a [`Message`] using `mlua`s `serde` support
/// and sends it to the core
fn lua_send(lua: &Lua, message: Value) -> Result<bool> {
    let app_data = lua.app_data_ref::<DsboxData>().unwrap();
    let message = match lua.from_value(message.clone()) {
        Ok(message) => message,
        Err(e) => {
            let raw_message = serde_json::to_string(&message).unwrap_or_default();
            app_data
                .send_event(ProcessEvent::SerializeError {
                    raw_message,
                    error: e.to_string(),
                })
                .ok();
            return Err(Error::RuntimeError(e.to_string()));
        }
    };
    Ok(app_data.send_event(ProcessEvent::Message(message)).is_ok())
}

/// waits for a single message to be received by the core
/// If no more messages are available `nil` is returned.
fn lua_recv(lua: &Lua, (timeout,): (Option<Number>,)) -> Result<Option<Value>> {
    let mut app_data = lua.app_data_mut::<DsboxData>().unwrap();
    let timeout = timeout.map(|t| Duration::from_secs_f64(t));
    let Some(command) = app_data.recv_command(timeout) else {
        return Ok(None);
    };
    match command {
        ProcessCommand::Deliver(message) => {
            let value = lua.to_value(&message)?;
            if let Some(message) = value.as_table() {
                let message_class: Table = get_dsbox(&lua, "Message")?;
                message_set_metatable(&lua, message, &message_class)?;
            }
            Ok(Some(value))
        }
        ProcessCommand::Abort => {
            Err(Error::ExternalError(Arc::new(Abort)))
        }
    }
}

/// returns an iterator that iterates over all received messages until there are no more
/// messages to be received (i.e. when the simulation shuts down)
fn lua_recv_iter(lua: &Lua, _: ()) -> Result<Value> {
    get_dsbox(lua, "recv")
}

fn lua_print(lua: &Lua, items: MultiValue) -> Result<bool> {
    let app_data = lua.app_data_ref::<DsboxData>().unwrap();
    let mut message = String::new();
    let mut first = true;
    for item in items {
        if first {
            first = false;
        } else {
            message.push('\t');
        }
        message.push_str(
            &lua.globals()
                .get::<Function>("tostring")?
                .call::<String>(item)?,
        );
    }
    Ok(app_data.send_event(ProcessEvent::Log(message)).is_ok())
}

fn lua_log(
    lua: &Lua,
    (message, label, color, _rest): (String, Option<String>, Option<String>, MultiValue),
) -> Result<bool> {
    let app_data = lua.app_data_ref::<DsboxData>().unwrap();
    let (Some(own_name), Some(core_name)) = (app_data.own_name(), app_data.core_name()) else {
        app_data.send_event(ProcessEvent::Log(message)).ok();
        return Ok(true);
    };

    let marker = if let Some(label) = label {
        let color = if let Some(color) = color {
            log_marker_color_from_str(&color)
        } else {
            None
        };
        Some(LogMarker { label, color })
    } else {
        None
    };

    Ok(app_data
        .send_event(ProcessEvent::Message(Message::new(
            own_name,
            core_name,
            None,
            LogMessage {
                text: message,
                marker,
            },
        )))
        .is_ok())
}

fn message_new(
    lua: &Lua,
    (message_class, src, dest, r#type, rest): (Table, Value, Value, Value, MultiValue),
) -> Result<Table> {
    let new_message = lua.create_table()?;
    new_message.set("src", src)?;
    new_message.set("dest", dest)?;
    let body = lua.create_table()?;
    body.set("type", r#type)?;
    merge_into_table(&body, rest)?;
    new_message.set("body", body)?;
    message_set_metatable(lua, &new_message, &message_class)?;
    Ok(new_message)
}

fn message_set_metatable(lua: &Lua, message: &Table, message_class: &Table) -> Result<()> {
    let metatable = lua.create_table()?;
    let tostring: Function = get_dsbox(lua, "to_json")?;
    metatable.set("__tostring", tostring)?;
    metatable.set("__index", message_class)?;
    message.set_metatable(Some(metatable))?;
    Ok(())
}

fn message_create_reply(
    lua: &Lua,
    (message, r#type, rest): (Table, Value, MultiValue),
) -> Result<Table> {
    let message_class = if let Some(metatable) = message.metatable() {
        metatable.get("__index")?
    } else {
        lua.create_table()?
    };
    let body: Table = message.get("body")?;
    let new_message = message_new(
        lua,
        (
            message_class,
            message.get("dest")?,
            message.get("src")?,
            r#type,
            MultiValue::new(),
        ),
    )?;
    let new_body: Table = new_message.get("body")?;
    new_body.set("in_reply_to", body.get::<Value>("id")?)?;
    merge_into_table(&new_body, rest)?;
    Ok(new_message)
}

fn message_reply(lua: &Lua, (message, r#type, rest): (Table, Value, MultiValue)) -> Result<bool> {
    let message = message_create_reply(&lua, (message, r#type, rest))?;
    message_send(lua, (message, MultiValue::new()))
}

fn message_send(lua: &Lua, (mut message, rest): (Table, MultiValue)) -> Result<bool> {
    let message_class: Table = get_dsbox(&lua, "Message")?;
    if message == message_class {
        let (src, dest, r#type, rest) = FromLuaMulti::from_lua_multi(rest, &lua)?;
        message = message_new(&lua, (message_class, src, dest, r#type, rest))?;
    }
    let message = message.into_lua(&lua)?;
    lua_send(lua, message)
}

fn lua_to_json(lua: &Lua, value: Value) -> Result<String> {
    let value: serde_json::Value = lua.from_value(value)?;
    Ok(serde_json::to_string(&value).unwrap())
}

fn merge_into_table(table: &Table, multi: MultiValue) -> Result<()> {
    for v in multi {
        let Some(t) = v.as_table() else {
            continue;
        };
        t.for_each(|k: Value, v: Value| table.set(k, v))?;
    }
    Ok(())
}

fn lua_array(lua: &Lua, table: Table) -> Result<Table> {
    table.set_metatable(Some(lua.array_metatable()))?;
    Ok(table)
}

fn get_dsbox<K: IntoLua, V: FromLua>(lua: &Lua, key: K) -> Result<V> {
    lua.globals()
        .get::<Table>("package")?
        .get::<Table>("loaded")?
        .get::<Table>("dsbox")?
        .get(key)
}

fn lua_clock(_: &Lua, _: ()) -> Result<u128> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap();
    Ok(elapsed.as_millis())
}

fn lua_sleep(_: &Lua, seconds: f64) -> Result<()> {
    std::thread::sleep(Duration::from_secs_f64(seconds));
    Ok(())
}

fn lua_exit(_: &Lua, exit_code: i32) -> Result<()> {
    Err(Error::ExternalError(Arc::new(Exit(exit_code))))
}

pub fn log_marker_color_from_str(color: &str) -> Option<LogMarkerColor> {
    match color {
        "black" | "Black" => Some(LogMarkerColor::Black),
        "red" | "Red" => Some(LogMarkerColor::Red),
        "green" | "Green" => Some(LogMarkerColor::Green),
        "yellow" | "Yellow" => Some(LogMarkerColor::Yellow),
        "blue" | "Blue" => Some(LogMarkerColor::Blue),
        "magenta" | "Magenta" => Some(LogMarkerColor::Magenta),
        "cyan" | "Cyan" => Some(LogMarkerColor::Cyan),
        "white" | "White" => Some(LogMarkerColor::White),
        "bright_black" | "BrightBlack" => Some(LogMarkerColor::BrightBlack),
        "bright_red" | "BrightRed" => Some(LogMarkerColor::BrightRed),
        "bright_green" | "BrightGreen" => Some(LogMarkerColor::BrightGreen),
        "bright_yellow" | "BrightYellow" => Some(LogMarkerColor::BrightYellow),
        "bright_blue" | "BrightBlue" => Some(LogMarkerColor::BrightBlue),
        "bright_magenta" | "BrightMagenta" => Some(LogMarkerColor::BrightMagenta),
        "bright_cyan" | "BrightCyan" => Some(LogMarkerColor::BrightCyan),
        "bright_white" | "BrightWhite" => Some(LogMarkerColor::BrightWhite),
        _ => None,
    }
}
