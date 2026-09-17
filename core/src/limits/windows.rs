//! How long a limit's window runs, worked out from the payload rather than
//! assumed from the plan.

use super::payload::RawLimit;
use chrono::{DateTime, Utc};

const FIVE_HOURS: u64 = 5 * 60 * 60;

const SEVEN_DAYS: u64 = 7 * 24 * 60 * 60;

/// Scoped limits carry the same reset instant as the window they belong to, but
/// computed a few microseconds apart.
const SAME_WINDOW_TOLERANCE_SECONDS: i64 = 5;

/// Which named window a limit belongs to, by its reset instant.
///
/// Falls back to the limit's kind when the named entries are absent, so a
/// payload that drops them still produces a marker.
pub(super) fn window_for(
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
