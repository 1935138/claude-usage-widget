//! Reading the access token Claude Code left on disk.
//!
//! The refresh token in the same file is never read or written: Claude Code
//! rotates it, and a second writer could log the user out of Claude Code.

use super::Unavailable;
use serde::Deserialize;
use std::path::{Path, PathBuf};

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

/// `.credentials.json` sits inside the config directory, next to `projects`.
pub fn credentials_path(projects_dir: &Path) -> Option<PathBuf> {
    let candidate = projects_dir.parent()?.join(".credentials.json");
    candidate.is_file().then_some(candidate)
}

/// Reads the access token, refusing one that is spent.
///
/// Errors deliberately carry no detail from the file: nothing here should be
/// able to reach a log or the UI.
pub(super) fn access_token(path: &Path, now_ms: i64) -> Result<String, Unavailable> {
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
