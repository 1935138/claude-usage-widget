//! The shapes Claude Code writes and the API answers with.
//!
//! Kept apart from the types the rest of the program works in: these mirror
//! someone else's JSON field for field, and change when that JSON changes.

use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CacheFile {
    #[serde(default)]
    pub(super) cached_usage_utilization: Option<Cached>,
    /// Sits beside the cache rather than inside it, and carries the plan tier.
    #[serde(default)]
    pub(super) oauth_account: Option<OauthAccount>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OauthAccount {
    /// Who the install is signed in as now, as opposed to whoever was signed in
    /// when the cache block was last written.
    #[serde(default)]
    pub(super) account_uuid: Option<String>,
    #[serde(default)]
    pub(super) email_address: Option<String>,
    #[serde(default)]
    pub(super) organization_name: Option<String>,
    #[serde(default)]
    pub(super) user_rate_limit_tier: Option<String>,
    /// Falls back for accounts that record no user tier: `claude_pro` reads as
    /// `Pro`, which is more use than the org's internal rate-limit tier name.
    #[serde(default)]
    pub(super) organization_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Cached {
    #[serde(default)]
    pub(super) fetched_at_ms: i64,
    #[serde(default)]
    pub(super) account_uuid: String,
    #[serde(default)]
    pub(super) utilization: Utilization,
}

#[derive(Debug, Default, Deserialize)]
pub struct Utilization {
    /// The named windows. Their keys are the API's own statement of how long
    /// each one runs, and both are present on every plan seen so far.
    #[serde(default)]
    pub(super) five_hour: Option<NamedWindow>,
    #[serde(default)]
    pub(super) seven_day: Option<NamedWindow>,
    #[serde(default)]
    pub(super) extra_usage: Option<RawExtraUsage>,
    #[serde(default)]
    pub(super) spend: Option<RawSpend>,
    /// Self-describing and already in display order, unlike the sibling
    /// `five_hour`/`seven_day` keys, so this is the only field read.
    #[serde(default)]
    pub(super) limits: Vec<RawLimit>,
}

#[derive(Debug, Deserialize)]
pub(super) struct NamedWindow {
    #[serde(default)]
    pub(super) resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawExtraUsage {
    #[serde(default)]
    pub(super) is_enabled: bool,
    #[serde(default)]
    pub(super) disabled_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawSpend {
    #[serde(default)]
    pub(super) percent: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawLimit {
    #[serde(default)]
    pub(super) kind: String,
    #[serde(default)]
    pub(super) percent: f64,
    #[serde(default)]
    pub(super) severity: Option<String>,
    #[serde(default)]
    pub(super) resets_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub(super) scope: Option<Scope>,
    #[serde(default)]
    pub(super) is_active: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct Scope {
    #[serde(default)]
    pub(super) model: Option<ScopeModel>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ScopeModel {
    #[serde(default)]
    pub(super) display_name: Option<String>,
}
