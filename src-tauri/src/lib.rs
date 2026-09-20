//! Tauri backend for the Claude Code usage widget.

mod cli;
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

/// Whether the window-state plugin has a position from a previous run.
///
/// There is no asking the plugin, so this looks for the file it keeps. Getting
/// the answer wrong is not fatal - it means a widget that opens top right once.
fn has_remembered_position(app: &tauri::AppHandle) -> bool {
    app.path()
        .app_config_dir()
        .map(|dir| {
            dir.join(tauri_plugin_window_state::DEFAULT_FILENAME)
                .exists()
        })
        .unwrap_or(false)
}

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
        // Position only. Size is worked out from the content, and restoring a
        // remembered one would fight that; visibility is the page's to decide,
        // and letting the plugin restore it would show the window before there
        // is anything in it. The plugin skips a remembered position that no
        // longer lands on any connected monitor, so unplugging a screen cannot
        // strand the widget off the edge of the desktop.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(tauri_plugin_window_state::StateFlags::POSITION)
                .build(),
        )
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                // Sized and placed while still hidden. The page shows it once it
                // has drawn, so nobody sees it at the config's placeholder
                // geometry or wearing the stylesheet's default corners.
                //
                // Only on a first run, though: after that the window has a place
                // the user put it, which the plugin has already restored by now.
                if !has_remembered_position(app.handle()) {
                    let _ = layout::place_initial(&window);
                }
                show_eventually(window);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::plan_limits,
            commands::login_state,
            commands::login,
            commands::open_install_docs,
            commands::fit_to_content,
            commands::load_settings,
            commands::save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
