//! Waiting out a 429 rather than walking straight back into it.
//!
//! Process-wide, because the limit applies to the account rather than to any
//! one call site.

use std::sync::Mutex;

/// The endpoint rate-limits. After a 429, wait this long, then twice that, and
/// so on, rather than walking straight back into it on the next tick.
const BACKOFF_START_MS: i64 = 5 * 60 * 1000;

const BACKOFF_MAX_MS: i64 = 60 * 60 * 1000;

/// When the next attempt is allowed, and how long the current backoff is.
/// Process-wide: the limit applies to the account, not to a call site.
static BACKOFF: Mutex<Option<(i64, i64)>> = Mutex::new(None);

/// How long the endpoint itself asked us to wait, if it said.
///
/// `Retry-After` is in seconds here. A value that is missing, unparseable or
/// absurd is ignored rather than trusted.
pub(super) fn retry_after_ms(response: &ureq::Response) -> Option<i64> {
    let seconds = response.header("retry-after")?.trim().parse::<i64>().ok()?;
    (seconds > 0).then(|| (seconds * 1000).min(BACKOFF_MAX_MS))
}

/// Whether a previous 429 is still being waited out.
pub(super) fn backing_off(now_ms: i64) -> bool {
    BACKOFF
        .lock()
        .ok()
        .and_then(|state| *state)
        .is_some_and(|(until, _)| now_ms < until)
}

/// Waits as long as the endpoint asked, or, failing an answer, twice as long as
/// last time up to the ceiling.
pub(super) fn begin_backoff(now_ms: i64, asked_ms: Option<i64>) {
    if let Ok(mut state) = BACKOFF.lock() {
        let next = asked_ms.unwrap_or_else(|| match *state {
            Some((_, previous)) => (previous * 2).min(BACKOFF_MAX_MS),
            None => BACKOFF_START_MS,
        });
        *state = Some((now_ms + next, next));
    }
}

pub(super) fn clear_backoff() {
    if let Ok(mut state) = BACKOFF.lock() {
        *state = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_then_stops_at_the_ceiling() {
        clear_backoff();
        begin_backoff(0, None);
        assert!(backing_off(BACKOFF_START_MS - 1));
        // The wait elapses.
        assert!(!backing_off(BACKOFF_START_MS + 1));

        // A second 429 waits twice as long.
        begin_backoff(0, None);
        assert!(backing_off(BACKOFF_START_MS * 2 - 1));

        for _ in 0..10 {
            begin_backoff(0, None);
        }
        assert!(!backing_off(BACKOFF_MAX_MS + 1));
        clear_backoff();
    }

    #[test]
    fn a_success_clears_the_backoff() {
        begin_backoff(0, None);
        clear_backoff();
        assert!(!backing_off(0));
    }

    #[test]
    fn the_endpoints_own_wait_wins_over_the_doubling() {
        clear_backoff();
        begin_backoff(0, Some(30_000));
        assert!(backing_off(29_999));
        assert!(!backing_off(30_001));
        clear_backoff();
    }
}
