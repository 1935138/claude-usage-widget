//! Reading and aggregating the JSONL session logs found by [`crate::discovery`].
//!
//! Every assistant turn Claude Code receives is appended as one JSON object
//! carrying `message.usage`. The same turn can appear more than once — a
//! session resumed under a different path, or one project tree visible through
//! two roots — so entries are keyed by `message.id` + `requestId` and counted
//! once.

use crate::discovery::{self, DataRoot};
use crate::limits::{self, Limits};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use walkdir::WalkDir;

/// Claude Code bills against a rolling five-hour window.
const BLOCK_HOURS: i64 = 5;
/// The weekly allowance is a rolling window too, not a calendar week.
const WEEK_DAYS: i64 = 7;

/// Placeholder model on records Claude Code synthesises locally (cancelled
/// turns, injected notices). They carry no real token cost.
const SYNTHETIC_MODEL: &str = "<synthetic>";

/// One line of a session log. Non-assistant lines deserialize with everything
/// absent and are skipped.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogLine {
    #[serde(default)]
    timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    message: Option<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

/// Token counts as reported by the API. Cache fields are absent on older
/// records, so all four default to zero.
#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    #[serde(default, alias = "input_tokens")]
    pub input: u64,
    #[serde(default, alias = "output_tokens")]
    pub output: u64,
    #[serde(default, alias = "cache_creation_input_tokens")]
    pub cache_write: u64,
    #[serde(default, alias = "cache_read_input_tokens")]
    pub cache_read: u64,
}

impl Usage {
    /// Every token the request was charged for.
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_write + self.cache_read
    }

    fn add(&mut self, other: &Usage) {
        self.input += other.input;
        self.output += other.output;
        self.cache_write += other.cache_write;
        self.cache_read += other.cache_read;
    }
}

/// A de-duplicated assistant turn, tagged with the root it was read from.
#[derive(Debug, Clone)]
struct Turn {
    /// Absent on a small number of malformed records.
    at: Option<DateTime<Utc>>,
    model: String,
    source: String,
    /// Which Claude Code session wrote this turn.
    session: Option<String>,
    usage: Usage,
}

/// A usage total plus the label it is grouped under.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub key: String,
    pub usage: Usage,
    pub total: u64,
}

/// What the widget renders.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// Roots that were read, for display and troubleshooting.
    pub sources: Vec<SourceInfo>,
    /// The most recently active session, whichever root it came from.
    pub current_session: Usage,
    /// Rolling five-hour window ending now.
    pub current_block: Usage,
    /// Midnight local time to now - not UTC, which would be hours out.
    pub today: Usage,
    /// Rolling seven days, matching how the weekly allowance is measured.
    pub last_7_days: Usage,
    /// The first of the month, local time, to now.
    pub this_month: Usage,
    pub all_time: Usage,
    pub by_model: Vec<Group>,
    /// Same totals split per root, so Windows and WSL are legible separately.
    pub by_source: Vec<Group>,
    /// Turns that were read twice and counted once.
    pub duplicates_skipped: u64,
    /// Plan-limit percentages as Claude Code last cached them, when available.
    /// Absent rather than zeroed, so the UI can say "unknown" instead of "0%".
    pub limits: Option<Limits>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub label: String,
    pub path: String,
    pub origin: discovery::Origin,
    pub sessions: u64,
}

/// Discover every root and aggregate all of them into one report.
pub fn collect() -> Report {
    let roots = discovery::discover();
    Report {
        // Percentages come from a different source than the logs: the cache
        // Claude Code writes after asking the API.
        limits: limits::load(),
        ..build(&roots)
    }
}

fn build(roots: &[DataRoot]) -> Report {
    let mut turns = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut duplicates_skipped = 0u64;
    let mut sources = Vec::new();

    for root in roots {
        let mut sessions = 0u64;
        for file in session_files(root) {
            sessions += 1;
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            for line in text.lines() {
                let Some((key, turn)) = parse_turn(line, &root.label) else {
                    continue;
                };
                if seen.insert(key) {
                    turns.push(turn);
                } else {
                    duplicates_skipped += 1;
                }
            }
        }
        sources.push(SourceInfo {
            label: root.label.clone(),
            path: root.projects_dir.display().to_string(),
            origin: root.origin,
            sessions,
        });
    }

    let now = Utc::now();
    let block_start = now - Duration::hours(BLOCK_HOURS);
    let week_start = now - Duration::days(WEEK_DAYS);
    let local_today = Local::now().date_naive();
    let day_start = local_day_start(local_today);
    let month_start = local_day_start(local_today.with_day(1).unwrap_or(local_today));

    let mut report = Report {
        sources,
        duplicates_skipped,
        ..Default::default()
    };
    let mut by_model: HashMap<String, Usage> = HashMap::new();
    let mut by_source: HashMap<String, Usage> = HashMap::new();

    for turn in &turns {
        report.all_time.add(&turn.usage);
        if let Some(at) = turn.at {
            if at >= day_start {
                report.today.add(&turn.usage);
            }
            if at >= block_start {
                report.current_block.add(&turn.usage);
            }
            if at >= week_start {
                report.last_7_days.add(&turn.usage);
            }
            if at >= month_start {
                report.this_month.add(&turn.usage);
            }
        }
        by_model.entry(turn.model.clone()).or_default().add(&turn.usage);
        by_source.entry(turn.source.clone()).or_default().add(&turn.usage);
    }

    report.current_session = newest_session_total(&turns);
    report.by_model = into_groups(by_model);
    report.by_source = into_groups(by_source);
    report
}

/// Midnight local time on `date`, as a UTC instant.
///
/// Turn timestamps are UTC, so the comparison has to happen in UTC - but the
/// boundary itself must be the user's midnight, not UTC's.
fn local_day_start(date: NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(0, 0, 0)
        .and_then(|naive| Local.from_local_datetime(&naive).earliest())
        .map(|local| local.with_timezone(&Utc))
        // A DST gap can swallow local midnight; falling back to the UTC day is
        // closer than dropping the boundary entirely.
        .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc())
}

/// Total for the session whose latest turn is the most recent one seen.
///
/// "Current" is defined by recency rather than by any liveness signal, since a
/// log file gives no indication of whether Claude Code still has it open.
fn newest_session_total(turns: &[Turn]) -> Usage {
    let mut latest: HashMap<&str, DateTime<Utc>> = HashMap::new();
    for turn in turns {
        if let (Some(session), Some(at)) = (turn.session.as_deref(), turn.at) {
            latest
                .entry(session)
                .and_modify(|seen| {
                    if at > *seen {
                        *seen = at;
                    }
                })
                .or_insert(at);
        }
    }
    let Some((newest, _)) = latest.into_iter().max_by_key(|&(_, at)| at) else {
        return Usage::default();
    };
    let mut total = Usage::default();
    for turn in turns {
        if turn.session.as_deref() == Some(newest) {
            total.add(&turn.usage);
        }
    }
    total
}

/// Groups sorted by total descending, so the UI can render them as-is.
fn into_groups(map: HashMap<String, Usage>) -> Vec<Group> {
    let mut groups: Vec<Group> = map
        .into_iter()
        .map(|(key, usage)| Group {
            key,
            total: usage.total(),
            usage,
        })
        .collect();
    groups.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.key.cmp(&b.key)));
    groups
}

/// Every `.jsonl` under a root. Depth is capped because the layout is
/// `projects/<project>/<session>.jsonl` and deeper trees are not ours.
fn session_files(root: &DataRoot) -> Vec<std::path::PathBuf> {
    WalkDir::new(&root.projects_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "jsonl"))
        .collect()
}

/// A billable turn and its de-duplication key, or `None` for any other line.
fn parse_turn(line: &str, source: &str) -> Option<(String, Turn)> {
    let parsed: LogLine = serde_json::from_str(line).ok()?;
    let message = parsed.message?;
    let usage = message.usage?;
    // Records without an id cannot be de-duplicated; the message id alone is
    // enough when a request id is missing.
    let id = message.id?;
    let model = message.model.unwrap_or_else(|| "unknown".to_string());
    if model == SYNTHETIC_MODEL {
        return None;
    }
    // One assistant message is written as several lines, one per content block
    // (text, thinking, tool_use), each repeating the *same* usage totals.
    // Keying on the message rather than the line is what keeps the total from
    // being inflated by roughly half.
    let key = format!("{id}:{}", parsed.request_id.unwrap_or_default());
    Some((
        key,
        Turn {
            at: parsed.timestamp,
            model,
            source: source.to_string(),
            session: parsed.session_id,
            usage,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    const LINE: &str = r#"{"type":"assistant","timestamp":"2026-09-11T00:30:00.000Z","requestId":"req_1","message":{"id":"msg_1","model":"claude-opus-5","usage":{"input_tokens":2,"output_tokens":407,"cache_creation_input_tokens":11407,"cache_read_input_tokens":21010}}}"#;

    #[test]
    fn parses_a_real_assistant_line() {
        let (key, turn) = parse_turn(LINE, "Windows").unwrap();
        assert_eq!(key, "msg_1:req_1");
        assert_eq!(turn.model, "claude-opus-5");
        assert_eq!(turn.usage.total(), 2 + 407 + 11407 + 21010);
    }

    #[test]
    fn skips_lines_without_usage() {
        for line in [
            r#"{"type":"mode","mode":"normal","sessionId":"s"}"#,
            r#"{"type":"user","message":{"role":"user","content":"hi"}}"#,
            "not json at all",
        ] {
            assert!(parse_turn(line, "Windows").is_none(), "{line}");
        }
    }

    #[test]
    fn same_turn_from_two_roots_is_keyed_identically() {
        // The dedup key must not depend on which root the line came from,
        // otherwise Windows and WSL would double-count a shared session.
        let (a, _) = parse_turn(LINE, "Windows").unwrap();
        let (b, _) = parse_turn(LINE, "WSL: <distro>").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn repeated_content_blocks_of_one_message_count_once() {
        // Claude Code writes one line per content block of the same assistant
        // message, each repeating the full usage. Counting lines instead of
        // messages inflated the real logs on this machine by ~45%.
        let roots = [DataRoot {
            projects_dir: std::path::PathBuf::from("unused"),
            origin: discovery::Origin::Native,
            label: "Windows".into(),
        }];
        let mut seen = HashSet::new();
        let mut counted = 0;
        for _ in 0..3 {
            let (key, _) = parse_turn(LINE, &roots[0].label).unwrap();
            if seen.insert(key) {
                counted += 1;
            }
        }
        assert_eq!(counted, 1);
    }

    #[test]
    fn synthetic_records_are_ignored() {
        let line = r#"{"timestamp":"2026-09-11T00:30:00.000Z","requestId":"r","message":{"id":"m","model":"<synthetic>","usage":{"input_tokens":0,"output_tokens":0}}}"#;
        assert!(parse_turn(line, "Windows").is_none());
    }

    #[test]
    fn a_turn_without_a_timestamp_still_counts_toward_all_time() {
        let line = r#"{"requestId":"r","message":{"id":"m","model":"claude-opus-5","usage":{"input_tokens":10,"output_tokens":5}}}"#;
        let (_, turn) = parse_turn(line, "Windows").unwrap();
        assert!(turn.at.is_none());
        assert_eq!(turn.usage.total(), 15);
    }

    /// Builds a turn directly, bypassing JSONL parsing.
    fn turn(session: &str, minutes_ago: i64, tokens: u64) -> Turn {
        Turn {
            at: Some(Utc::now() - Duration::minutes(minutes_ago)),
            model: "claude-opus-5".into(),
            source: "Windows".into(),
            session: Some(session.into()),
            usage: Usage { input: tokens, ..Default::default() },
        }
    }

    #[test]
    fn current_session_is_the_most_recently_active_one() {
        let turns = vec![
            turn("old", 600, 100),
            turn("old", 500, 100),
            turn("new", 5, 7),
            turn("new", 1, 3),
        ];
        // Only the newest session counts, and every one of its turns does.
        assert_eq!(newest_session_total(&turns).total(), 10);
    }

    #[test]
    fn current_session_ignores_turns_with_no_session_id() {
        let mut turns = vec![turn("only", 10, 42)];
        turns.push(Turn { session: None, ..turn("ignored", 1, 999) });
        assert_eq!(newest_session_total(&turns).total(), 42);
    }

    #[test]
    fn current_session_is_zero_with_nothing_to_go_on() {
        assert_eq!(newest_session_total(&[]).total(), 0);
    }

    #[test]
    fn local_day_start_is_midnight_in_the_local_zone() {
        let today = Local::now().date_naive();
        let start = local_day_start(today);
        // Converting back must land on local midnight of the same date.
        let back = start.with_timezone(&Local);
        assert_eq!(back.date_naive(), today);
        assert_eq!((back.time().hour(), back.time().minute()), (0, 0));
        assert!(start <= Utc::now());
    }

    #[test]
    fn groups_are_sorted_by_total_descending() {
        let mut map = HashMap::new();
        map.insert("small".to_string(), Usage { input: 1, ..Default::default() });
        map.insert("big".to_string(), Usage { input: 99, ..Default::default() });
        let groups = into_groups(map);
        assert_eq!(groups[0].key, "big");
        assert_eq!(groups[1].key, "small");
    }
}
