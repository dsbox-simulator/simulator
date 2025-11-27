use dsbox_core::core::Core;
use crate::dsbox::Commands;
use tauri::Manager;
use tokio::sync::RwLock;
pub mod cli;
mod dsbox;
mod util;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(args: cli::Cli) {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle()
                    .plugin(tauri_plugin_log::Builder::default().skip_logger().build())?;
            }
            let test_command = args.test_command.map(Core::split_command)
                .unwrap_or_default();
            let server_command = args.server_command.map(Core::make_command)
                .unwrap_or_default();
            app.manage(RwLock::new(dsbox::DsboxState::new(
                Commands {
                    test_command,
                    server_command,
                },
                args.lua_unsafe,
            )));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            dsbox::restart,
            dsbox::subscribe_events,
            dsbox::break_,
            dsbox::step,
            dsbox::resume,
            dsbox::current_commands,
            dsbox::deliver,
            dsbox::drop,
            util::find_interpreter,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
