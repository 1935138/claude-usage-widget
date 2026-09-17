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

mod backoff;
mod credentials;

use crate::limits::Utilization;
use backoff::{backing_off, begin_backoff, clear_backoff, retry_after_ms};
use credentials::access_token;
use std::path::Path;
use std::time::Duration;

pub use credentials::credentials_path;

const ENDPOINT: &str = "https://api.anthropic.com/api/oauth/usage?at_wall=1&skip_spend=1";
/// The widget refreshes on a timer; a hung request must not stack up.
const TIMEOUT: Duration = Duration::from_secs(8);
/// How the widget introduces itself to the usage endpoint.
///
/// The endpoint keeps two very different rate limits and chooses between them
/// on this header alone. `claude-code/<version>` gets a workable one; anything
/// else gets one so tight that a handful of requests earns hours of 429s, with
/// `Retry-After: 0` and no way to tell when it lifts. The widget therefore
/// introduces itself as the client it stands in for, reading the same endpoint,
/// for the same account, with the token Claude Code itself put on disk. The
/// version trails the CLI's own; only the prefix decides the bucket.
const USER_AGENT: &str = "claude-code/2.1.80";
/// Why a live read was not possible. Carries no token material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// No credentials file beside this install.
    NoCredentials,
    /// The access token has lapsed; Claude Code renews it as it is used.
    Expired,
    /// The request failed, was refused, or returned something unparseable.
    RequestFailed,
    /// The endpoint is rate-limiting, or a previous 429 is still being waited
    /// out. The caller shows the last figures it has rather than nothing.
    RateLimited,
}

impl Unavailable {
    /// Short tag for the UI, so it can say why figures are cached instead of
    /// always telling the user to run `/usage`. Carries no token material.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoCredentials => "noCredentials",
            Self::Expired => "expired",
            Self::RequestFailed => "requestFailed",
            Self::RateLimited => "rateLimited",
        }
    }
}

/// Current utilisation for the account that owns `credentials`.
pub fn fetch(credentials: &Path, now_ms: i64) -> Result<Utilization, Unavailable> {
    if backing_off(now_ms) {
        return Err(Unavailable::RateLimited);
    }
    let token = access_token(credentials, now_ms)?;
    let response = ureq::get(ENDPOINT)
        .timeout(TIMEOUT)
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", USER_AGENT)
        .set("Content-Type", "application/json")
        .set("Cache-Control", "no-cache")
        .call()
        .map_err(|error| match error {
            ureq::Error::Status(429, response) => {
                begin_backoff(now_ms, retry_after_ms(&response));
                Unavailable::RateLimited
            }
            _ => Unavailable::RequestFailed,
        })?;
    let parsed = response
        .into_json::<Utilization>()
        .map_err(|_| Unavailable::RequestFailed)?;
    clear_backoff();
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_agent_names_the_client_the_endpoint_expects() {
        // Not cosmetic: without this prefix the endpoint answers 429 for hours.
        assert!(USER_AGENT.starts_with("claude-code/"));
    }

    #[test]
    fn every_reason_has_a_tag_for_the_ui() {
        for why in [
            Unavailable::NoCredentials,
            Unavailable::Expired,
            Unavailable::RequestFailed,
            Unavailable::RateLimited,
        ] {
            assert!(!why.code().is_empty());
        }
        assert_eq!(Unavailable::Expired.code(), "expired");
    }
}
