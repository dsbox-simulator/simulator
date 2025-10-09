use tauri::Manager;
pub mod cli;
mod dsbox;
mod util;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(args: cli::Cli) {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.manage(dsbox::DsboxState::new(args));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            dsbox::subscribe_events,
            dsbox::restart,
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
