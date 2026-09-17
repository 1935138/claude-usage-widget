//! Everything the page can call, and nothing else.
//!
//! Each of these is one `invoke` from the frontend. Work that touches the disk
//! or the network goes to a blocking thread: one of the files can sit across
//! the WSL boundary, where a read is a network round trip.

use crate::{layout, settings};
use claude_usage_core::{discovery, limits, live};

/// Plan-limit meters for every account a cache was found for, freshest first.
///
/// Reading a handful of small JSON files, but one of them can sit across the
/// WSL boundary, so it stays off the UI thread.
#[tauri::command]
pub async fn plan_limits() -> Vec<limits::Limits> {
    tauri::async_runtime::spawn_blocking(limits::load_all)
        .await
        .unwrap_or_default()
}

/// Whether this machine's own Claude Code install has never signed in.
///
/// Only the install the widget itself is running on is checked - a WSL
/// install with no credentials is not this widget's to log into, and quota is
/// per account, so leaving one signed out there is a deliberate choice, not
/// something to nag about here.
#[tauri::command]
pub async fn needs_login() -> bool {
    tauri::async_runtime::spawn_blocking(|| {
        let here = discovery::discover()
            .into_iter()
            .find(|root| root.origin != discovery::Origin::Wsl);
        match here {
            Some(root) => live::credentials_path(&root.projects_dir).is_none(),
            None => true,
        }
    })
    .await
    .unwrap_or(false)
}

/// Opens `claude login` in its own console window.
///
/// Run through `cmd` rather than invoked directly: a Node-installed CLI
/// resolves to a `.cmd` shim, which Windows will not execute as the target of
/// `CreateProcess` on its own. Left attached to a new console rather than
/// hidden, since the login flow may print a URL to open by hand.
#[tauri::command]
pub fn login() -> Result<(), String> {
    std::process::Command::new("cmd")
        .args(["/C", "claude", "login"])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// What the widget should show.
#[tauri::command]
pub fn load_settings(app: tauri::AppHandle) -> claude_usage_core::settings::Settings {
    settings::load(&app)
}

#[tauri::command]
pub fn save_settings(
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
pub fn fit_to_content(window: tauri::WebviewWindow, height: f64) -> Result<(), String> {
    layout::fit(&window, height).map_err(|e| e.to_string())?;
    // The page measures itself as soon as it has drawn, so this is also the
    // first moment the window is worth looking at.
    if !window.is_visible().unwrap_or(true) {
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}
