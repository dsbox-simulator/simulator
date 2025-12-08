use tauri::Manager;
use tokio::sync::RwLock;
pub mod args;
pub mod dsbox;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(args: args::Args) {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle()
                    .plugin(tauri_plugin_log::Builder::default().skip_logger().build())?;
            }
            let state = dsbox::DsboxState::new(
                args.test_command.unwrap_or_default(),
                args.server_command.unwrap_or_default().join(" "),
                args.common.lua_unsafe,
            );
            app.manage(RwLock::new(state));
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
            dsbox::interpreters::find_interpreter,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
