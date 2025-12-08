use crate::dsbox::remote_control::RemoteControl;
use dsbox_core::{Builder, RunnerCommand};

pub fn guess_runner(command: &str) -> RunnerCommand {
    let args = RunnerCommand::split_command(command)
        .unwrap_or_else(|| command.split(" ").map(|s| s.to_owned()).collect());
    if cfg!(feature = "wasm") && args[0].ends_with(".wasm") {
        RunnerCommand::new("wasm", args)
    } else if cfg!(feature = "lua") && args[0].ends_with(".lua") {
        RunnerCommand::new("lua", args)
    } else {
        RunnerCommand::new("native", args)
    }
}

pub fn register_runners(
    builder: Builder,
    lua_unsafe: bool,
    remote_control: RemoteControl,
) -> Builder {
    #[cfg(feature = "lua")]
    fn lua_runner(builder: Builder, lua_unsafe: bool) -> Builder {
        builder.register_runner("lua", dsbox_runner_lua::LuaRunner::new(lua_unsafe))
    }
    #[cfg(not(feature = "lua"))]
    fn lua_runner(builder: Builder, _: bool) -> Builder {
        builder
    }
    #[cfg(feature = "wasm")]
    fn wasm_runner(builder: Builder) -> Builder {
        builder.register_runner("wasm", dsbox_runner_wasm::WasmRunner::new())
    }
    #[cfg(not(feature = "wasm"))]
    fn wasm_runner(builder: Builder) -> Builder {
        builder
    }

    let builder = builder
        .register_runner("native", dsbox_runner_native::NativeRunner)
        .register_runner("remote_control", remote_control);

    let builder = lua_runner(builder, lua_unsafe);
    wasm_runner(builder)
}
