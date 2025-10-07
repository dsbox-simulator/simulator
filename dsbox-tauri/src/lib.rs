use tauri::Manager;
pub mod cli;
mod dsbox;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(args: cli::Cli) {
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");

    tauri::Builder::default()
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
