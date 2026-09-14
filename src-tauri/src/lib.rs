//! Tauri backend for the Claude Code usage widget.

mod layout;
mod settings;

pub use claude_usage_core::limits;

use tauri::Manager;

/// Plan-limit meters for every account a cache was found for, freshest first.
///
/// Reading a handful of small JSON files, but one of them can sit across the
/// WSL boundary, so it stays off the UI thread.
#[tauri::command]
async fn plan_limits() -> Vec<limits::Limits> {
    tauri::async_runtime::spawn_blocking(limits::load_all)
        .await
        .unwrap_or_default()
}

/// What the widget should show.
#[tauri::command]
fn load_settings(app: tauri::AppHandle) -> claude_usage_core::settings::Settings {
    settings::load(&app)
}

#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    settings: claude_usage_core::settings::Settings,
) -> Result<(), String> {
    settings::save(&app, &settings)
}

/// Resize the window to the height the page measured for its content.
///
/// The window is undecorated and fixed-size, so the page cannot grow a
/// scrollbar-free layout on its own; it reports how tall it rendered and the
/// clamping against the display's work area happens here.
#[tauri::command]
fn fit_to_content(window: tauri::WebviewWindow, height: f64) -> Result<(), String> {
    layout::fit(&window, height).map_err(|e| e.to_string())
}

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
            plan_limits,
            fit_to_content,
            load_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
