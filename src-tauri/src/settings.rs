//! Persisting [`Settings`] to the app's config directory.
//!
//! JSON on disk, so it survives a reset of the webview's own storage and can be
//! edited by hand.

use claude_usage_core::settings::Settings;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("no config directory: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {dir:?}: {e}"))?;
    Ok(dir.join("settings.json"))
}

/// Reads the stored settings, falling back to the defaults.
///
/// A missing or unreadable file is not worth surfacing - the widget then simply
/// shows everything, which is what a first run does anyway.
pub fn load(app: &AppHandle) -> Settings {
    path(app)
        .ok()
        .and_then(|file| std::fs::read_to_string(file).ok())
        .and_then(|text| Settings::parse(&text))
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let file = path(app)?;
    let text =
        serde_json::to_string_pretty(&settings.clone().normalized()).map_err(|e| e.to_string())?;
    std::fs::write(&file, text).map_err(|e| format!("could not write {file:?}: {e}"))
}
