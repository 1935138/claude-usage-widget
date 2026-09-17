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

use crate::discovery::{self, DataRoot};
use crate::live;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheFile {
    #[serde(default)]
    cached_usage_utilization: Option<Cached>,
    /// Sits beside the cache rather than inside it, and carries the plan tier.
    #[serde(default)]
    oauth_account: Option<OauthAccount>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OauthAccount {
    /// Who the install is signed in as now, as opposed to whoever was signed in
    /// when the cache block was last written.
    #[serde(default)]
    account_uuid: Option<String>,
    #[serde(default)]
    email_address: Option<String>,
    #[serde(default)]
    organization_name: Option<String>,
    #[serde(default)]
    user_rate_limit_tier: Option<String>,
    /// Falls back for accounts that record no user tier: `claude_pro` reads as
    /// `Pro`, which is more use than the org's internal rate-limit tier name.
    #[serde(default)]
    organization_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cached {
    #[serde(default)]
    fetched_at_ms: i64,
    #[serde(default)]
    account_uuid: String,
    #[serde(default)]
    utilization: Utilization,
}

#[derive(Debug, Default, Deserialize)]
pub struct Utilization {
    /// The named windows. Their keys are the API's own statement of how long
    /// each one runs, and both are present on every plan seen so far.
    #[serde(default)]
    five_hour: Option<NamedWindow>,
    #[serde(default)]
    seven_day: Option<NamedWindow>,
    #[serde(default)]
    extra_usage: Option<RawExtraUsage>,
    #[serde(default)]
    spend: Option<RawSpend>,
    /// Self-describing and already in display order, unlike the sibling
    /// `five_hour`/`seven_day` keys, so this is the only field read.
    #[serde(default)]
    limits: Vec<RawLimit>,
}

#[derive(Debug, Deserialize)]
struct NamedWindow {
    #[serde(default)]
    resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct RawExtraUsage {
    #[serde(default)]
    is_enabled: bool,
    #[serde(default)]
    disabled_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSpend {
    #[serde(default)]
    percent: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawLimit {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    percent: f64,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    resets_at: Option<DateTime<Utc>>,
    #[serde(default)]
    scope: Option<Scope>,
    #[serde(default)]
    is_active: bool,
}

#[derive(Debug, Deserialize)]
struct Scope {
    #[serde(default)]
    model: Option<ScopeModel>,
}

#[derive(Debug, Deserialize)]
struct ScopeModel {
    #[serde(default)]
    display_name: Option<String>,
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

const FIVE_HOURS: u64 = 5 * 60 * 60;
const SEVEN_DAYS: u64 = 7 * 24 * 60 * 60;
/// Scoped limits carry the same reset instant as the window they belong to, but
/// computed a few microseconds apart.
const SAME_WINDOW_TOLERANCE_SECONDS: i64 = 5;

/// Which named window a limit belongs to, by its reset instant.
///
/// Falls back to the limit's kind when the named entries are absent, so a
/// payload that drops them still produces a marker.
fn window_for(
    raw: &RawLimit,
    five_hour: Option<DateTime<Utc>>,
    seven_day: Option<DateTime<Utc>>,
) -> Option<u64> {
    let resets = raw.resets_at?;
    let matches = |other: Option<DateTime<Utc>>| {
        other.is_some_and(|o| (resets - o).num_seconds().abs() <= SAME_WINDOW_TOLERANCE_SECONDS)
    };
    if matches(five_hour) {
        Some(FIVE_HOURS)
    } else if matches(seven_day) {
        Some(SEVEN_DAYS)
    } else if raw.kind == "session" {
        Some(FIVE_HOURS)
    } else if raw.kind.starts_with("weekly") {
        Some(SEVEN_DAYS)
    } else {
        None
    }
}

/// `default_claude_max_5x` reads as `Max 5x`, `claude_pro` as `Pro`; the
/// prefixes are plumbing.
fn plan_name(tier: &str) -> Option<String> {
    let tier = tier.trim();
    if tier.is_empty() {
        return None;
    }
    let bare = tier
        .strip_prefix("default_claude_")
        .or_else(|| tier.strip_prefix("default_"))
        .or_else(|| tier.strip_prefix("claude_"))
        .unwrap_or(tier);
    let mut words = bare.split('_').map(|w| {
        let mut cs = w.chars();
        match cs.next() {
            Some(first) => first.to_uppercase().collect::<String>() + cs.as_str(),
            None => String::new(),
        }
    });
    let first = words.next()?;
    Some(words.fold(first, |acc, w| acc + " " + &w))
}

fn scoped_model(scope: &Option<Scope>) -> Option<&str> {
    scope.as_ref()?.model.as_ref()?.display_name.as_deref()
}

/// Wording matched to `/usage`, falling back to the raw kind for any limit type
/// added later rather than dropping it.
fn label_for(kind: &str, model: Option<&str>) -> String {
    match (kind, model) {
        ("session", _) => "Current session".to_string(),
        ("weekly_all", _) => "Current week (all models)".to_string(),
        ("weekly_scoped", Some(model)) => format!("Current week ({model})"),
        ("weekly_scoped", None) => "Current week (scoped)".to_string(),
        (other, Some(model)) => format!("{} ({model})", other.replace('_', " ")),
        (other, None) => other.replace('_', " "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_match_the_usage_command() {
        assert_eq!(label_for("session", None), "Current session");
        assert_eq!(label_for("weekly_all", None), "Current week (all models)");
        assert_eq!(
            label_for("weekly_scoped", Some("Fable")),
            "Current week (Fable)"
        );
    }

    #[test]
    fn an_unknown_limit_kind_is_still_shown() {
        assert_eq!(label_for("monthly_thing", None), "monthly thing");
        assert_eq!(
            label_for("monthly_thing", Some("Opus")),
            "monthly thing (Opus)"
        );
    }

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
    fn plan_tier_reads_as_a_plan_name() {
        assert_eq!(
            plan_name("default_claude_max_5x").as_deref(),
            Some("Max 5x")
        );
        assert_eq!(plan_name("claude_pro").as_deref(), Some("Pro"));
        assert_eq!(plan_name("claude_team").as_deref(), Some("Team"));
        assert_eq!(plan_name("default_raven").as_deref(), Some("Raven"));
        assert_eq!(plan_name("  "), None);
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

    fn limit(kind: &str, resets: &str) -> RawLimit {
        RawLimit {
            kind: kind.to_string(),
            percent: 0.0,
            severity: None,
            resets_at: Some(resets.parse().unwrap()),
            scope: None,
            is_active: false,
        }
    }

    #[test]
    fn a_window_is_recognised_by_its_reset_instant() {
        let five: DateTime<Utc> = "2026-09-11T09:59:59.639712Z".parse().unwrap();
        let seven: DateTime<Utc> = "2026-09-18T05:59:59.639734Z".parse().unwrap();
        let session = limit("session", "2026-09-11T09:59:59.639712Z");
        assert_eq!(
            window_for(&session, Some(five), Some(seven)),
            Some(FIVE_HOURS)
        );
        let weekly = limit("weekly_all", "2026-09-18T05:59:59.639734Z");
        assert_eq!(
            window_for(&weekly, Some(five), Some(seven)),
            Some(SEVEN_DAYS)
        );
    }

    #[test]
    fn a_scoped_limit_matches_its_window_despite_the_microsecond_drift() {
        // Real payloads compute these a few hundred microseconds apart.
        let seven: DateTime<Utc> = "2026-09-18T05:59:59.639734Z".parse().unwrap();
        let scoped = limit("weekly_scoped", "2026-09-18T05:59:59.639993Z");
        assert_eq!(window_for(&scoped, None, Some(seven)), Some(SEVEN_DAYS));
    }

    #[test]
    fn the_kind_stands_in_when_the_named_windows_are_missing() {
        assert_eq!(
            window_for(&limit("session", "2026-09-11T09:59:59Z"), None, None),
            Some(FIVE_HOURS)
        );
        assert_eq!(
            window_for(&limit("weekly_scoped", "2026-09-18T05:59:59Z"), None, None),
            Some(SEVEN_DAYS)
        );
        // An unfamiliar kind gets no marker rather than a guessed one.
        assert_eq!(
            window_for(&limit("monthly_thing", "2026-09-18T05:59:59Z"), None, None),
            None
        );
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
