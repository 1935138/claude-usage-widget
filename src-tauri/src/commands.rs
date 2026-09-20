//! Everything the page can call, and nothing else.
//!
//! Each of these is one `invoke` from the frontend. Work that touches the disk
//! or the network goes to a blocking thread: one of the files can sit across
//! the WSL boundary, where a read is a network round trip.

use crate::{cli, layout, settings};
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

/// What this machine can offer, as far as showing figures goes.
///
/// Three answers rather than two: a machine with no Claude Code on it needs
/// installing, not signing in, and telling someone to sign in when there is
/// nothing to sign in with is the complaint this exists to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ClaudeState {
    /// Signed in here already.
    Ok,
    /// Claude Code is installed but has never signed in.
    Needed,
    /// No Claude Code on this machine at all.
    NotInstalled,
}

/// Whether this machine's own Claude Code install is signed in, and failing
/// that whether it is installed.
///
/// Only the install the widget itself is running on is checked - a WSL install
/// with no credentials is not this widget's to log into, and quota is per
/// account, so leaving one signed out there is a deliberate choice, not
/// something to nag about here.
#[tauri::command]
pub async fn login_state() -> ClaudeState {
    tauri::async_runtime::spawn_blocking(|| {
        let signed_in = discovery::discover()
            .into_iter()
            .find(|root| root.origin != discovery::Origin::Wsl)
            .is_some_and(|root| live::credentials_path(&root.projects_dir).is_some());
        if signed_in {
            ClaudeState::Ok
        } else if cli::locate().is_some() {
            ClaudeState::Needed
        } else {
            ClaudeState::NotInstalled
        }
    })
    .await
    .unwrap_or(ClaudeState::Ok)
}

/// Opens `claude auth login` in its own console window.
///
/// The CLI is located first: it is routinely absent from the widget's `PATH`,
/// and a shell asked for it by bare name closes its console on the error
/// before anyone can read it.
#[tauri::command]
pub fn login() -> Result<(), String> {
    let exe = cli::locate().ok_or("no Claude Code on this machine")?;
    cli::spawn_login(&exe).map_err(|e| e.to_string())
}

/// Opens the install instructions in the default browser.
#[tauri::command]
pub fn open_install_docs() -> Result<(), String> {
    cli::open_install_docs().map_err(|e| e.to_string())
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
