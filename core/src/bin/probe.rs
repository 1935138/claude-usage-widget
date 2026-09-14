//! Prints what the widget would read, without opening a window.
//!
//! Useful for checking path discovery on a machine before wiring up the UI:
//!   cargo run --bin probe

use claude_usage_core::{discovery, limits, usage};

fn main() {
    // `probe --json` prints exactly what the UI receives, which is the easiest
    // way to keep the TypeScript types honest.
    if std::env::args().any(|a| a == "--json") {
        let report = usage::collect();
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        return;
    }

    let roots = discovery::discover();
    println!("discovered {} log root(s):", roots.len());
    for root in &roots {
        println!(
            "  [{:?}] {} -> {}",
            root.origin,
            root.label,
            root.projects_dir.display()
        );
    }

    if roots.is_empty() {
        println!("\nno Claude Code logs found");
        return;
    }

    let report = usage::collect();
    println!("\nsessions read per source:");
    for source in &report.sources {
        println!("  {:<24} {} session(s)", source.label, source.sessions);
    }
    println!("\nduplicate turns skipped: {}", report.duplicates_skipped);
    println!("current session:  {:>12} tokens", report.current_session.total());
    println!("current 5h block: {:>12} tokens", report.current_block.total());
    println!("today (local):    {:>12} tokens", report.today.total());
    println!("last 7 days:      {:>12} tokens", report.last_7_days.total());
    println!("this month:       {:>12} tokens", report.this_month.total());
    println!("all time:         {:>12} tokens", report.all_time.total());

    let all = limits::load_all();
    if all.is_empty() {
        println!("\nplan limits: no cache found");
    }
    for l in &all {
        {
            println!("\nplan limits [{}] from {} ({}):", l.account, l.source,
                if l.live { "LIVE" } else { "cached" });
            println!("  {} | {} | {}",
                l.email.clone().unwrap_or_else(|| "?".into()),
                l.plan.clone().unwrap_or_else(|| "?".into()),
                l.organization.clone().unwrap_or_else(|| "?".into()));
            println!("  fetched {}", l.fetched_at);
            if let Some(x) = &l.extra_usage {
                println!("  extra usage: enabled={} reason={:?} spend={:?}", x.enabled, x.disabled_reason, x.percent);
            }
            for m in &l.meters {
                let resets = m.resets_at.map(|r| r.to_rfc3339()).unwrap_or_else(|| "-".into());
                println!("  {:<28} {:>5.0}%  {}{}", m.label, m.percent,
                    if m.is_active { "active  " } else { "        " }, resets);
            }
        }
    }

    println!("\nby source:");
    for group in &report.by_source {
        println!("  {:<24} {:>12}", group.key, group.total);
    }
    println!("\nby model:");
    for group in &report.by_model {
        println!("  {:<24} {:>12}", group.key, group.total);
    }
}
