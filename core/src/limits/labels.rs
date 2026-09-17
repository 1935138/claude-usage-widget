//! Turning what the API reports into what the card reads out.

use super::payload::Scope;

/// `default_claude_max_5x` reads as `Max 5x`, `claude_pro` as `Pro`; the
/// prefixes are plumbing.
pub(super) fn plan_name(tier: &str) -> Option<String> {
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

pub(super) fn scoped_model(scope: &Option<Scope>) -> Option<&str> {
    scope.as_ref()?.model.as_ref()?.display_name.as_deref()
}

/// Wording matched to `/usage`, falling back to the raw kind for any limit type
/// added later rather than dropping it.
pub(super) fn label_for(kind: &str, model: Option<&str>) -> String {
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
}
