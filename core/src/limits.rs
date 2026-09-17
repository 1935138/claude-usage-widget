//! Reading Claude Code's cached plan-limit utilisation.
//!
//! The percentages `/usage` prints are not derivable from the session logs:
//! they come from the API, and Claude Code caches its last answer in
//! `~/.claude.json` under `cachedUsageUtilization`. This module reads that
//! cache; it never talks to the network.
//!
//! Two consequences follow, and both are surfaced rather than hidden:
//!
//! * The numbers are only as fresh as the last Claude Code run on that machine,
//!   so [`Limits::fetched_at`] is reported for the UI to age.
//! * Quota is per *account*. Where several installs each hold a cache they may
//!   be signed in as different accounts, so the caches are never merged - the
//!   most recently fetched one wins outright.

mod labels;
mod payload;
mod windows;

use crate::discovery::{self, DataRoot};
use crate::live;
use chrono::{DateTime, TimeZone, Utc};
use labels::{label_for, plan_name, scoped_model};
use payload::{CacheFile, Cached};
use serde::Serialize;
use std::path::{Path, PathBuf};
use windows::window_for;

pub use payload::Utilization;

/// How far up from a `projects` directory the sibling `~/.claude.json` can sit.
/// `<home>/.claude/projects` needs three; `<home>/.config/claude/projects` four.
const MAX_ANCESTORS: usize = 5;

/// One limit bar, mirroring a row of `/usage`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meter {
    /// Raw limit kind, e.g. `session` or `weekly_scoped`. Stable enough to key
    /// settings on, unlike the label.
    pub kind: String,
    /// Rendered label, e.g. `Current week (Fable)`.
    pub label: String,
    pub percent: f64,
    /// `normal`, or a raised level the UI must call out in words as well as colour.
    pub severity: String,
    pub resets_at: Option<DateTime<Utc>>,
    /// How long this limit's window runs, in seconds.
    ///
    /// Derived from the payload rather than assumed: each limit's reset instant
    /// is matched against the `five_hour` and `seven_day` entries sitting beside
    /// it, which is how the API itself names the two window lengths.
    pub window_seconds: Option<u64>,
    /// Whether this is the window currently being consumed.
    pub is_active: bool,
}

/// Whether pay-as-you-go credits can absorb overflow once a limit is hit.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraUsage {
    pub enabled: bool,
    /// Why it is off, when it is, e.g. `out_of_credits`.
    pub disabled_reason: Option<String>,
    /// Share of the spend cap used, only meaningful while enabled.
    pub percent: Option<f64>,
}

/// One install's cache, and enough provenance to judge it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub meters: Vec<Meter>,
    pub fetched_at: DateTime<Utc>,
    /// Which install the cache came from, e.g. `WSL: <distro>`.
    pub source: String,
    /// Short account id. Installs can be signed in as different accounts, and
    /// the quota shown belongs to exactly one of them.
    pub account: String,
    /// Signed-in email. The clearest way to tell accounts apart - two installs
    /// on one machine can belong to different people entirely.
    pub email: Option<String>,
    /// Organization the account sits in, for the tooltip.
    pub organization: Option<String>,
    /// Plan tier, e.g. `Max 5x` or `Pro`.
    pub plan: Option<String>,
    /// Whether these figures came from the API just now rather than from the
    /// cache Claude Code last wrote.
    pub live: bool,
    pub extra_usage: Option<ExtraUsage>,
    /// Why the figures are cached rather than live, as a
    /// [`live::Unavailable::code`] tag. `None` while they are live.
    pub reason: Option<String>,
}

impl Limits {
    /// Records why a live read did not happen. Ignored once figures are live.
    fn because(mut self, why: live::Unavailable) -> Self {
        if !self.live {
            self.reason = Some(why.code().to_string());
        }
        self
    }
}

/// Who an install is signed in as, as of now.
///
/// Read from `oauthAccount` rather than from the cache block. The cache records
/// whoever was signed in when it was last written, which on a long-lived
/// install can be a different account entirely - and a stale id there is enough
/// to make one login look like two.
struct Identity {
    account: String,
    email: Option<String>,
    organization: Option<String>,
    plan: Option<String>,
}

impl Identity {
    fn of(file: &CacheFile) -> Self {
        let uuid = file
            .oauth_account
            .as_ref()
            .and_then(|a| a.account_uuid.clone())
            .or_else(|| {
                file.cached_usage_utilization
                    .as_ref()
                    .map(|c| c.account_uuid.clone())
            })
            .unwrap_or_default();
        let oauth = file.oauth_account.as_ref();
        Self {
            account: uuid.chars().take(8).collect(),
            email: oauth.and_then(|a| a.email_address.clone()),
            organization: oauth.and_then(|a| a.organization_name.clone()),
            plan: oauth.and_then(|a| {
                a.user_rate_limit_tier
                    .as_deref()
                    .and_then(plan_name)
                    .or_else(|| a.organization_type.as_deref().and_then(plan_name))
            }),
        }
    }
}

/// The freshest plan-limit cache across every install on this machine.
pub fn load() -> Option<Limits> {
    load_all().into_iter().next()
}

/// Every account a cache was found for, freshest first.
///
/// Installs on one machine can be signed in as different accounts - quota is
/// per account, so these are offered as alternatives rather than combined. Where
/// two installs share an account only the fresher cache is kept - see
/// [`freshest_per_account`] for what counts as the same account.
pub fn load_all() -> Vec<Limits> {
    from_roots(&discovery::discover())
}

fn from_roots(roots: &[DataRoot]) -> Vec<Limits> {
    let now_ms = Utc::now().timestamp_millis();
    let mut found: Vec<Limits> = roots
        .iter()
        .filter_map(|root| one_root(root, now_ms))
        .collect();
    freshest_per_account(&mut found);
    found
}

/// One install's figures: live where the credentials allow it, cached only
/// where they do not.
///
/// The cache is a fallback, not a precondition. An install whose `.claude.json`
/// has never held a `cachedUsageUtilization` - or holds one written months ago
/// under a different account - still reports live figures, because the identity
/// comes from `oauthAccount` and the meters from the API. Where the live read
/// cannot happen, the reason travels with the cached figures so the UI can say
/// what is actually wrong.
fn one_root(root: &DataRoot, now_ms: i64) -> Option<Limits> {
    let file = cache_path(&root.projects_dir).and_then(parse)?;
    let identity = Identity::of(&file);
    let cached = cached_limits(file.cached_usage_utilization, &identity, &root.label);

    let Some(credentials) = live::credentials_path(&root.projects_dir) else {
        return cached.map(|limits| limits.because(live::Unavailable::NoCredentials));
    };
    match live::fetch(&credentials, now_ms) {
        Ok(fresh) => match live_limits(fresh, &identity, &root.label) {
            // The request asks the endpoint to skip spend, so the credit state
            // the tooltip shows can only have come from the cache.
            Some(mut limits) => {
                limits.extra_usage = limits
                    .extra_usage
                    .or_else(|| cached.and_then(|stale| stale.extra_usage));
                Some(limits)
            }
            None => cached,
        },
        Err(why) => cached.map(|limits| limits.because(why)),
    }
}

/// Orders caches freshest first and drops every repeat of an account.
///
/// Identity is the signed-in email where there is one, since the `account_uuid`
/// Claude Code records can differ between installs signed into the very same
/// account. A cache carrying no email falls back to its `account_uuid`, so two
/// anonymous installs are still kept apart.
fn freshest_per_account(found: &mut Vec<Limits>) {
    found.sort_by_key(|limits| std::cmp::Reverse(limits.fetched_at));
    let mut seen = std::collections::HashSet::new();
    found.retain(|limits| {
        let key = limits
            .email
            .clone()
            .unwrap_or_else(|| limits.account.clone());
        seen.insert(key)
    });
}

/// `~/.claude.json` sits beside the config directory, not inside it, so walk up
/// from `projects` until a sibling cache appears.
fn cache_path(projects_dir: &Path) -> Option<PathBuf> {
    projects_dir
        .ancestors()
        .take(MAX_ANCESTORS)
        .map(|dir| dir.join(".claude.json"))
        .find(|candidate| candidate.is_file())
}

fn parse(path: PathBuf) -> Option<CacheFile> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<CacheFile>(&text).ok()
}

/// The figures Claude Code last wrote, if it has written any.
fn cached_limits(cached: Option<Cached>, identity: &Identity, source: &str) -> Option<Limits> {
    let mut cached = cached?;
    let extra_usage = extra_usage_of(&mut cached.utilization);
    Some(Limits {
        meters: meters_from(cached.utilization)?,
        fetched_at: Utc.timestamp_millis_opt(cached.fetched_at_ms).single()?,
        source: source.to_string(),
        account: identity.account.clone(),
        email: identity.email.clone(),
        organization: identity.organization.clone(),
        plan: identity.plan.clone(),
        live: false,
        extra_usage,
        reason: None,
    })
}

/// Figures straight from the API, owing nothing to the cache.
fn live_limits(mut fresh: Utilization, identity: &Identity, source: &str) -> Option<Limits> {
    let extra_usage = extra_usage_of(&mut fresh);
    Some(Limits {
        meters: meters_from(fresh)?,
        fetched_at: Utc::now(),
        source: source.to_string(),
        account: identity.account.clone(),
        email: identity.email.clone(),
        organization: identity.organization.clone(),
        plan: identity.plan.clone(),
        live: true,
        extra_usage,
        reason: None,
    })
}

/// Lifts the credit state out of a payload, cached or fresh, leaving the limits
/// behind for [`meters_from`].
fn extra_usage_of(utilization: &mut Utilization) -> Option<ExtraUsage> {
    let raw = utilization.extra_usage.take();
    let spend = utilization.spend.take();
    raw.map(|raw| ExtraUsage {
        enabled: raw.is_enabled,
        disabled_reason: raw.disabled_reason,
        percent: spend.and_then(|s| s.percent),
    })
}

/// The cached figures alone, for tests that work from a file on disk.
#[cfg(test)]
fn read(path: &Path, source: &str) -> Option<Limits> {
    let file = parse(path.to_path_buf())?;
    let identity = Identity::of(&file);
    cached_limits(file.cached_usage_utilization, &identity, source)
}

/// Turns a utilisation payload - cached or freshly fetched, they share a shape -
/// into display meters. `None` when it carries no limits at all.
fn meters_from(utilization: Utilization) -> Option<Vec<Meter>> {
    let five_hour = utilization.five_hour.and_then(|w| w.resets_at);
    let seven_day = utilization.seven_day.and_then(|w| w.resets_at);
    let meters: Vec<Meter> = utilization
        .limits
        .into_iter()
        .map(|raw| Meter {
            window_seconds: window_for(&raw, five_hour, seven_day),
            label: label_for(&raw.kind, scoped_model(&raw.scope)),
            kind: raw.kind,
            percent: raw.percent,
            severity: raw.severity.unwrap_or_else(|| "normal".to_string()),
            resets_at: raw.resets_at,
            is_active: raw.is_active,
        })
        .collect();
    (!meters.is_empty()).then_some(meters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_shape_claude_code_writes() {
        let dir = std::env::temp_dir().join("cuw-limits-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".claude.json");
        std::fs::write(
            &path,
            r#"{"cachedUsageUtilization":{"fetchedAtMs":1789115359076,"accountUuid":"00000000-0000-0000","utilization":{"limits":[
                {"kind":"session","percent":46,"severity":"normal","resets_at":"2026-09-11T09:59:59.639712+00:00","scope":null,"is_active":true},
                {"kind":"weekly_scoped","percent":5,"severity":"normal","resets_at":"2026-09-18T05:59:59.639993+00:00","scope":{"model":{"id":null,"display_name":"Fable"}},"is_active":false}]}}}"#,
        )
        .unwrap();

        let limits = read(&path, "WSL: <distro>").unwrap();
        assert_eq!(limits.account, "00000000");
        assert_eq!(limits.meters.len(), 2);
        assert_eq!(limits.meters[0].kind, "session");
        assert_eq!(limits.meters[0].label, "Current session");
        assert_eq!(limits.meters[0].percent, 46.0);
        assert!(limits.meters[0].is_active);
        assert_eq!(limits.meters[1].label, "Current week (Fable)");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn identity_follows_the_current_login_not_the_cached_one() {
        // A long-lived install keeps whatever account_uuid was current when the
        // cache was last written. Trusting it made one login look like two.
        let file: CacheFile = serde_json::from_str(
            r#"{"oauthAccount":{"accountUuid":"c8abb3bc-eb26","emailAddress":"you@example.com",
                "organizationName":"Acme","userRateLimitTier":"default_claude_max_5x"},
                "cachedUsageUtilization":{"accountUuid":"9fb0e902-1b77","fetchedAtMs":1}}"#,
        )
        .unwrap();
        let identity = Identity::of(&file);
        assert_eq!(identity.account, "c8abb3bc");
        assert_eq!(identity.email.as_deref(), Some("you@example.com"));
        assert_eq!(identity.plan.as_deref(), Some("Max 5x"));
    }

    #[test]
    fn identity_falls_back_to_the_cached_account_when_there_is_no_oauth_block() {
        let file: CacheFile =
            serde_json::from_str(r#"{"cachedUsageUtilization":{"accountUuid":"9fb0e902-1b77"}}"#)
                .unwrap();
        assert_eq!(Identity::of(&file).account, "9fb0e902");
        assert!(Identity::of(&file).email.is_none());
    }

    #[test]
    fn a_reason_is_recorded_for_cached_figures_only() {
        let base = Limits {
            meters: vec![],
            fetched_at: Utc.timestamp_millis_opt(0).single().unwrap(),
            source: "Windows".into(),
            account: "c8abb3bc".into(),
            email: None,
            organization: None,
            plan: None,
            live: false,
            extra_usage: None,
            reason: None,
        };
        let cached = base.clone().because(live::Unavailable::Expired);
        assert_eq!(cached.reason.as_deref(), Some("expired"));
        // Live figures owe nothing to a failed read elsewhere.
        let live = Limits { live: true, ..base }.because(live::Unavailable::RateLimited);
        assert!(live.reason.is_none());
    }

    #[test]
    fn one_account_per_entry_keeping_the_fresher_cache() {
        // Two installs signed in as the same account must not both be offered.
        let make = |account: &str, ms: i64| Limits {
            meters: vec![],
            fetched_at: Utc.timestamp_millis_opt(ms).single().unwrap(),
            source: format!("src{ms}"),
            account: account.to_string(),
            email: None,
            organization: None,
            plan: None,
            live: false,
            extra_usage: None,
            reason: None,
        };
        let mut found = vec![make("aaa", 100), make("bbb", 300), make("aaa", 500)];
        freshest_per_account(&mut found);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].account, "aaa");
        assert_eq!(found[0].source, "src500");
    }

    #[test]
    fn same_email_with_different_account_uuids_is_still_one_entry() {
        // Claude Code can record a different `account_uuid` per install for the
        // very same login, so email - not account_uuid - is what must dedupe.
        let make = |account: &str, email: Option<&str>, ms: i64| Limits {
            meters: vec![],
            fetched_at: Utc.timestamp_millis_opt(ms).single().unwrap(),
            source: format!("src{ms}"),
            account: account.to_string(),
            email: email.map(str::to_string),
            organization: None,
            plan: None,
            live: ms == 500,
            extra_usage: None,
            reason: None,
        };
        let mut found = vec![
            make("9fb0e902", Some("a@example.com"), 500),
            make("c8abb3bc", Some("a@example.com"), 100),
        ];
        freshest_per_account(&mut found);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].account, "9fb0e902");
        assert!(found[0].live);
    }

    #[test]
    fn a_file_without_the_cache_yields_nothing() {
        let dir = std::env::temp_dir().join("cuw-limits-empty");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".claude.json");
        std::fs::write(&path, r#"{"projects":{}}"#).unwrap();
        assert!(read(&path, "x").is_none());
        std::fs::remove_file(&path).ok();
    }
}
