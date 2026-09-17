//! Tauri backend for the Claude Code usage widget.

mod commands;
mod layout;
mod settings;

pub use claude_usage_core::limits;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                // Give the window a sane size and corner before it is shown,
                // so it does not flash at the config's placeholder geometry.
                let _ = layout::place_initial(&window);
                let _ = window.show();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::plan_limits,
            commands::needs_login,
            commands::login,
            commands::fit_to_content,
            commands::load_settings,
            commands::save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
