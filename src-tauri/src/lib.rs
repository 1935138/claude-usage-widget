//! Tauri backend for the Claude Code usage widget.

mod commands;
mod layout;
mod settings;

pub use claude_usage_core::limits;

use tauri::Manager;

/// How long to wait for the page before showing the window regardless.
///
/// The page asks to be shown as soon as it has drawn. If it never manages to -
/// a broken bundle, a webview that failed to start - a window that never
/// appears is worse than one that appears empty.
const SHOW_ANYWAY_AFTER: std::time::Duration = std::time::Duration::from_secs(3);

fn show_eventually(window: tauri::WebviewWindow) {
    std::thread::spawn(move || {
        std::thread::sleep(SHOW_ANYWAY_AFTER);
        if !window.is_visible().unwrap_or(true) {
            let _ = window.show();
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                // Sized and placed while still hidden. The page shows it once it
                // has drawn, so nobody sees it at the config's placeholder
                // geometry or wearing the stylesheet's default corners.
                let _ = layout::place_initial(&window);
                show_eventually(window);
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
