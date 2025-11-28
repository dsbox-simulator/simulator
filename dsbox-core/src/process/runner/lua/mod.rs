mod appdata;
mod dsbox_module;
#[cfg(windows)]
mod windows;

use crate::process::ProcessEvent;
use crate::process::runner::lua::appdata::DsboxData;
use crate::process::runner::{CommandReceiver, EventSender, Runner};
use mlua::{Error, Lua, LuaOptions, StdLib, Table, Value};
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub struct LuaRunner {
    luarocks_path: Option<String>,
    luarocks_cpath: Option<String>,
    allow_os_libs: bool,
}

/// a custom error type that can be used to communicate an early return with exit code
/// from a lua script.
#[derive(Debug, Copy, Clone)]
pub(self) struct Exit(i32);

/// a custom error type that can be used to communicate an abort from the core
#[derive(Debug, Copy, Clone)]
pub(self) struct Abort;

impl LuaRunner {
    pub async fn new(allow_os_libs: bool) -> Self {
        let (path, cpath) = if let Ok((path, cpath)) = Self::query_luarocks_path().await {
            (Some(path), Some(cpath))
        } else {
            (None, None)
        };
        Self {
            luarocks_path: path,
            luarocks_cpath: cpath,
            allow_os_libs,
        }
    }

    async fn query_luarocks_path() -> tokio::io::Result<(String, String)> {
        let path = tokio::process::Command::new("luarocks")
            .args(["path", "--lr-path"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;
        let cpath = tokio::process::Command::new("luarocks")
            .args(["path", "--lr-cpath"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;
        let path = String::from_utf8_lossy(&path.stdout).to_string();
        let cpath = String::from_utf8_lossy(&cpath.stdout).to_string();
        Ok((path, cpath))
    }

    /// creates a new [`Lua`] instance and sets it up with some globals, like `args`, send and receive
    /// functions, a Message class etc.
    fn setup_lua(&self, lua_file: &Path, args: &[String]) -> mlua::Result<Lua> {
        let libs = StdLib::TABLE
            | StdLib::STRING
            | StdLib::UTF8
            | StdLib::COROUTINE
            | StdLib::MATH
            | StdLib::PACKAGE;
        let lua = if self.allow_os_libs {
            unsafe { Lua::unsafe_new_with(libs | StdLib::OS | StdLib::IO, LuaOptions::new()) }
        } else {
            Lua::new_with(libs, LuaOptions::new())?
        };

        dsbox_module::init_dsbox_module(&lua, args)?;

        self.set_package_paths(&lua, lua_file)?;

        Ok(lua)
    }

    fn set_package_paths(&self, lua: &Lua, lua_file: &Path) -> mlua::Result<()> {
        let package: Table = lua.globals().get("package")?;

        macro_rules! join_path {
                ($($p:expr),*) => {{
                    let mut path = PathBuf::new();
                    $(path.push($p);)*
                    path
                }};
            }

        // setup module search paths
        let version: String = lua.globals().get("_VERSION")?;
        let mut path = if let Some(path) = &self.luarocks_path {
            path.clone()
        } else {
            package.get("path")?
        };
        let mut cpath = if let Some(cpath) = &self.luarocks_cpath {
            cpath.clone()
        } else {
            package.get("cpath")?
        };
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
        push_cpath(join_path!(
            source_path,
            "lua_modules",
            "lib64",
            "lua",
            &version[4..],
            "?.so"
        ));

        Self::set_paths(&package, path, cpath)?;
        Ok(())
    }

    #[cfg(not(windows))]
    fn set_paths(package: &Table, path: String, cpath: String) -> mlua::Result<()> {
        package.set("path", path)?;
        package.set("cpath", cpath)?;
        Ok(())
    }

    #[cfg(windows)]
    fn set_paths(package: &Table, path: String, cpath: String) -> mlua::Result<()> {
        // on windows, convert the paths into the active codepage first, to avoid issues
        // with umlauts in paths
        package.set("path", windows::convert_to_acp(&path))?;
        package.set("cpath", windows::convert_to_acp(&cpath))?;
        Ok(())
    }
}

impl Runner for LuaRunner {
    fn run(
        &mut self,
        args: Vec<String>,
        sender: EventSender,
        receiver: CommandReceiver,
    ) -> impl Future<Output = i32> + 'static {
        let lua_file = PathBuf::from(&args[0]);
        let lua = self.setup_lua(&lua_file, &args[1..]);

        async move {
            let lua = match lua {
                Ok(lua) => lua,
                Err(e) => {
                    sender.send(ProcessEvent::Log(e.to_string())).await.ok();
                    return -1;
                }
            };

            lua.set_app_data(DsboxData::new(sender, receiver));
            let chunk = lua.load(lua_file.clone());

            let result = tokio::task::spawn_blocking(|| {
                chunk.call(()).map(|v: Value| v.as_i32().unwrap_or(0))
            })
            .await
            .expect("embedded lua interpreter panicked");

            let app_data = lua.remove_app_data::<DsboxData>().unwrap();
            let exit_code = match extract_exit_code(result) {
                Ok(exit_code) => exit_code,
                Err(e) => {
                    let error_message = e.to_string();
                    let message = format!(
                        "script `{}` exited with an error: {error_message}",
                        lua_file.display()
                    );
                    app_data
                        .into_sender()
                        .send(ProcessEvent::Log(message.clone()))
                        .await
                        .ok();
                    log::warn!("{}", message);
                    -1
                }
            };
            exit_code
        }
    }
}

fn extract_exit_code(result: Result<i32, Error>) -> Result<i32, Error> {
    match result {
        Ok(exit_code) => Ok(exit_code),
        Err(Error::CallbackError { traceback, cause }) => {
            if let Error::ExternalError(std_error) = cause.as_ref() {
                if let Some(exit) = std_error.downcast_ref::<Exit>() {
                    return Ok(exit.0);
                } else if std_error.is::<Abort>() {
                    return Ok(0);
                }
            }
            Err(Error::CallbackError { traceback, cause })
        }
        Err(Error::ExternalError(std_error)) => {
            if let Some(exit) = std_error.downcast_ref::<Exit>() {
                Ok(exit.0)
            } else if std_error.is::<Abort>() {
                Ok(0)
            } else {
                Err(Error::ExternalError(std_error))
            }
        }
        Err(e) => Err(e),
    }
}

impl std::fmt::Display for Exit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Exit({})", self.0)
    }
}
impl std::error::Error for Exit {}

impl std::fmt::Display for Abort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Abort")
    }
}
impl std::error::Error for Abort {}
