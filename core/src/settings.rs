//! What the widget shows.
//!
//! Kept here rather than in the Tauri shell so the defaults and their
//! forward-compatibility can be tested without a desktop toolchain.

use serde::{Deserialize, Serialize};

/// Refresh intervals outside this range are clamped. The floor keeps a
/// hand-edited settings file from hammering the API, which does rate-limit;
/// the ceiling keeps an absent-minded one from looking broken.
pub const MIN_REFRESH_SECONDS: u32 = 60;
pub const MAX_REFRESH_SECONDS: u32 = 3600;
/// Five minutes. The endpoint returns 429 under repeated polling, and the
/// shortest window being tracked is five hours, so there is nothing to gain
/// from asking more often.
pub const DEFAULT_REFRESH_SECONDS: u32 = 300;

/// Where the clock sits, and whether it is there at all.
///
/// Kept as a string rather than an enum so an unrecognised value degrades to
/// the default instead of failing the whole file and resetting every other
/// setting with it.
pub const CLOCK_POSITIONS: [&str; 4] = ["off", "left", "center", "right"];
pub const DEFAULT_CLOCK: &str = "right";

/// Which parts of the widget are shown.
///
/// `#[serde(default)]` on the container fills any missing field from [`Default`]
/// rather than from `bool`'s own default - otherwise a settings file written by
/// an older version would silently switch off every item added since.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// The rolling five-hour meter, `kind == "session"`.
    pub session: bool,
    /// The weekly meters, both the all-models one and any per-model ones.
    pub weekly: bool,
    /// The line saying when the cache was written and which install wrote it.
    pub provenance: bool,
    /// The marker showing how far through the window the clock has got.
    pub pace: bool,
    /// Short id of the account whose meters to show. `None` follows whichever
    /// cache is freshest, which is also the fallback if the id disappears.
    pub account: Option<String>,
    /// Seconds between automatic refreshes; `0` means manual only.
    pub refresh_seconds: u32,
    /// One of [`CLOCK_POSITIONS`].
    pub clock: String,
}

impl Settings {
    /// Reads a settings file, tolerating a UTF-8 BOM.
    ///
    /// Editors on Windows routinely add one and `serde_json` rejects it, which
    /// would silently drop the user back to defaults — a nasty outcome for a
    /// file the documentation invites people to edit.
    pub fn parse(text: &str) -> Option<Self> {
        serde_json::from_str::<Self>(text.trim_start_matches('\u{feff}'))
            .ok()
            .map(Self::normalized)
    }

    /// Brings a hand-edited or older file into the supported range.
    pub fn normalized(mut self) -> Self {
        if self.refresh_seconds != 0 {
            self.refresh_seconds = self
                .refresh_seconds
                .clamp(MIN_REFRESH_SECONDS, MAX_REFRESH_SECONDS);
        }
        if !CLOCK_POSITIONS.contains(&self.clock.as_str()) {
            self.clock = DEFAULT_CLOCK.to_string();
        }
        self
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            session: true,
            weekly: true,
            provenance: true,
            pace: true,
            account: None,
            refresh_seconds: DEFAULT_REFRESH_SECONDS,
            clock: DEFAULT_CLOCK.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_is_shown_by_default() {
        let s = Settings::default();
        assert!(s.session && s.weekly && s.provenance && s.pace);
        assert!(s.account.is_none());
    }

    #[test]
    fn a_utf8_bom_does_not_wipe_the_file() {
        let text = "\u{feff}{\"session\":false,\"clock\":\"left\"}";
        let parsed = Settings::parse(text).expect("a BOM must not make the file unreadable");
        assert!(!parsed.session);
        assert_eq!(parsed.clock, "left");
    }

    #[test]
    fn parse_normalizes_as_it_reads() {
        let parsed = Settings::parse(r#"{"refreshSeconds":1,"clock":"diagonal"}"#).unwrap();
        assert_eq!(parsed.refresh_seconds, MIN_REFRESH_SECONDS);
        assert_eq!(parsed.clock, DEFAULT_CLOCK);
    }

    #[test]
    fn parse_rejects_what_is_not_settings_at_all() {
        assert!(Settings::parse("not json").is_none());
    }

    #[test]
    fn an_absent_interval_takes_the_default() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.refresh_seconds, DEFAULT_REFRESH_SECONDS);
    }

    #[test]
    fn a_hand_edited_interval_is_clamped_into_range() {
        let fast = Settings { refresh_seconds: 5, ..Default::default() }.normalized();
        assert_eq!(fast.refresh_seconds, MIN_REFRESH_SECONDS);
        let slow = Settings { refresh_seconds: 99_999, ..Default::default() }.normalized();
        assert_eq!(slow.refresh_seconds, MAX_REFRESH_SECONDS);
    }

    #[test]
    fn manual_only_survives_normalization() {
        // Zero is meaningful - it is not "unset", it is "do not poll".
        let manual = Settings { refresh_seconds: 0, ..Default::default() }.normalized();
        assert_eq!(manual.refresh_seconds, 0);
    }

    #[test]
    fn an_unknown_clock_position_falls_back_without_losing_other_settings() {
        let s: Settings = serde_json::from_str(r#"{"clock":"diagonal","session":false}"#).unwrap();
        let s = s.normalized();
        assert_eq!(s.clock, DEFAULT_CLOCK);
        // The point of the string: the bad value must not reset the rest.
        assert!(!s.session);
    }

    #[test]
    fn every_offered_clock_position_survives_normalization() {
        for position in CLOCK_POSITIONS {
            let s = Settings { clock: position.to_string(), ..Default::default() }.normalized();
            assert_eq!(s.clock, position);
        }
    }

    #[test]
    fn a_chosen_account_survives_a_round_trip() {
        let s = Settings {
            account: Some("00000000".into()),
            ..Default::default()
        };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.account.as_deref(), Some("00000000"));
    }

    #[test]
    fn fields_absent_from_an_older_file_stay_on() {
        // The danger is `#[serde(default)]` reaching for `bool::default()`,
        // which would turn newly added items off for existing users.
        let s: Settings = serde_json::from_str(r#"{"session":false}"#).unwrap();
        assert!(!s.session);
        assert!(s.weekly && s.provenance && s.pace);
    }

    #[test]
    fn round_trips_through_json() {
        let s = Settings {
            weekly: false,
            ..Default::default()
        };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert!(!back.weekly);
        assert!(back.session && back.provenance);
    }
}
