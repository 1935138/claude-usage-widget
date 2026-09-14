//! Fetching plan-limit utilisation from the API instead of Claude Code's cache.
//!
//! Claude Code only rewrites `cachedUsageUtilization` when `/usage` runs, so the
//! cached figures can be days stale. This module asks the same endpoint Claude
//! Code asks, using the OAuth access token already sitting in
//! `~/.claude/.credentials.json`.
//!
//! **The refresh token is never touched.** Claude Code rotates it, and a widget
//! writing that file could log the user out of Claude Code itself. The access
//! token is read and used as-is; once it expires the caller falls back to the
//! cache until Claude Code refreshes it in the course of normal use.

use crate::limits::Utilization;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

const ENDPOINT: &str = "https://api.anthropic.com/api/oauth/usage?at_wall=1&skip_spend=1";
/// The widget refreshes on a timer; a hung request must not stack up.
const TIMEOUT: Duration = Duration::from_secs(8);
/// Stop using a token shortly before it lapses, rather than racing the clock.
const EXPIRY_MARGIN_MS: i64 = 60_000;

#[derive(Debug, Deserialize)]
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    oauth: Option<Oauth>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Oauth {
    access_token: String,
    #[serde(default)]
    expires_at: i64,
}

/// Why a live read was not possible. Carries no token material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// No credentials file beside this install.
    NoCredentials,
    /// The access token has lapsed; Claude Code renews it as it is used.
    Expired,
    /// The request failed, was refused, or returned something unparseable.
    RequestFailed,
}

/// `.credentials.json` sits inside the config directory, next to `projects`.
pub fn credentials_path(projects_dir: &Path) -> Option<PathBuf> {
    let candidate = projects_dir.parent()?.join(".credentials.json");
    candidate.is_file().then_some(candidate)
}

/// Current utilisation for the account that owns `credentials`.
pub fn fetch(credentials: &Path, now_ms: i64) -> Result<Utilization, Unavailable> {
    let token = access_token(credentials, now_ms)?;
    let response = ureq::get(ENDPOINT)
        .timeout(TIMEOUT)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .set("Cache-Control", "no-cache")
        .call()
        .map_err(|_| Unavailable::RequestFailed)?;
    response
        .into_json::<Utilization>()
        .map_err(|_| Unavailable::RequestFailed)
}

/// Reads the access token, refusing one that is spent.
///
/// Errors deliberately carry no detail from the file: nothing here should be
/// able to reach a log or the UI.
fn access_token(path: &Path, now_ms: i64) -> Result<String, Unavailable> {
    let text = std::fs::read_to_string(path).map_err(|_| Unavailable::NoCredentials)?;
    let oauth = serde_json::from_str::<CredentialsFile>(&text)
        .map_err(|_| Unavailable::NoCredentials)?
        .oauth
        .ok_or(Unavailable::NoCredentials)?;
    if oauth.expires_at > 0 && oauth.expires_at - EXPIRY_MARGIN_MS <= now_ms {
        return Err(Unavailable::Expired);
    }
    Ok(oauth.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(name: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("cuw-live-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn reads_a_live_token() {
        let path = write(
            "ok.json",
            r#"{"claudeAiOauth":{"accessToken":"tok","expiresAt":2000000}}"#,
        );
        assert_eq!(access_token(&path, 1_000_000).unwrap(), "tok");
    }

    #[test]
    fn refuses_a_spent_token() {
        let path = write(
            "old.json",
            r#"{"claudeAiOauth":{"accessToken":"tok","expiresAt":1000000}}"#,
        );
        assert_eq!(access_token(&path, 1_000_000), Err(Unavailable::Expired));
    }

    #[test]
    fn refuses_a_token_inside_the_expiry_margin() {
        // Valid for another 30s: not long enough to be worth starting a request.
        let path = write(
            "edge.json",
            r#"{"claudeAiOauth":{"accessToken":"tok","expiresAt":1030000}}"#,
        );
        assert_eq!(access_token(&path, 1_000_000), Err(Unavailable::Expired));
    }

    #[test]
    fn a_file_without_oauth_is_not_credentials() {
        let path = write("empty.json", r#"{"other":true}"#);
        assert_eq!(
            access_token(&path, 1_000_000),
            Err(Unavailable::NoCredentials)
        );
    }

    #[test]
    fn a_missing_file_is_not_credentials() {
        assert_eq!(
            access_token(Path::new("/nonexistent-creds-xyz.json"), 0),
            Err(Unavailable::NoCredentials)
        );
    }
}
