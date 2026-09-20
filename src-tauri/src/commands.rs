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
pub async fn plan_limits(app: tauri::AppHandle) -> Vec<limits::Limits> {
    let extra = settings::load(&app).extra_accounts;
    tauri::async_runtime::spawn_blocking(move || limits::load_all_with(&extra))
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
    cli::spawn_login(&exe, None).map_err(|e| e.to_string())
}

/// Opens the install instructions in the default browser.
#[tauri::command]
pub fn open_install_docs() -> Result<(), String> {
    cli::open_install_docs().map_err(|e| e.to_string())
}

/// Where the widget keeps the config directories it signs in itself.
///
/// Beside its own settings rather than in the home directory: these are the
/// widget's to make and to remove, and a home directory is not the place for
/// something nobody asked to see. Clearing the widget's data clears these with
/// it, which costs a sign-in and nothing else.
fn accounts_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("accounts"))
        .map_err(|e| format!("no config directory: {e}"))
}

/// Starts a sign-in for one more account, in a directory of its own.
///
/// The directory is not recorded yet. A console the user closes without
/// finishing would otherwise leave an entry that can never show anything, so
/// the settings file learns about it only once [`confirm_account`] finds
/// credentials in it.
#[tauri::command]
pub fn add_account(app: tauri::AppHandle) -> Result<String, String> {
    let current = settings::load(&app);
    if current.extra_accounts.len() >= claude_usage_core::settings::MAX_EXTRA_ACCOUNTS {
        return Err("too many accounts".into());
    }
    let exe = cli::locate().ok_or("no Claude Code on this machine")?;

    // Named for the moment it was started, in milliseconds: unique enough for
    // a button nobody can click twice that fast, and no dependency for it.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis())
        .unwrap_or(0);
    let dir = accounts_dir(&app)?.join(stamp.to_string());
    cli::spawn_login(&exe, Some(&dir)).map_err(|e| e.to_string())?;
    Ok(dir.display().to_string())
}

/// Whether the sign-in started in `dir` has landed.
#[tauri::command]
pub fn account_signed_in(dir: String) -> bool {
    cli::signed_in(std::path::Path::new(&dir))
}

/// Records a directory whose sign-in finished, so its account is read from now
/// on. Refuses one that never signed in, which is what an abandoned console
/// leaves behind.
#[tauri::command]
pub fn confirm_account(app: tauri::AppHandle, dir: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&dir);
    if !cli::signed_in(&path) {
        return Err("that directory has no sign-in in it".into());
    }
    let mut current = settings::load(&app);
    if !current.extra_accounts.contains(&path) {
        current.extra_accounts.push(path);
    }
    settings::save(&app, &current)
}

/// Forgets an added account, and removes its directory if the widget made it.
///
/// A directory the widget did not create is left alone however it was named:
/// someone who pointed the settings file at an install of their own is not
/// expecting the widget to delete it.
#[tauri::command]
pub fn remove_account(app: tauri::AppHandle, dir: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&dir);
    let mut current = settings::load(&app);
    current.extra_accounts.retain(|kept| kept != &path);
    settings::save(&app, &current)?;

    if path.starts_with(accounts_dir(&app)?) {
        let _ = std::fs::remove_dir_all(&path);
    }
    Ok(())
}

/// Clears out directories from sign-ins that never finished.
///
/// Only ones the widget made, only ones the settings file does not name, and
/// only ones holding no credentials. A console still open when this runs is at
/// worst sent back to the start.
pub fn sweep_abandoned(app: &tauri::AppHandle) {
    let Ok(root) = accounts_dir(app) else { return };
    let kept = settings::load(app).extra_accounts;
    let Ok(entries) = std::fs::read_dir(&root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && !kept.contains(&path) && !cli::signed_in(&path) {
            let _ = std::fs::remove_dir_all(&path);
        }
    }
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
