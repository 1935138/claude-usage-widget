// Turning values into the words and numbers the card shows.
//
// Pure: each of these takes what it needs, so nothing here reaches for the
// settings or the DOM.

import type { ExtraUsage, Limits, Meter } from "./types";

/** A cache older than this is called out; it only refreshes when Claude Code runs. */
export const STALE_MS = 60 * 60_000;

export const esc = (s: string): string =>
  s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

/**
 * Every time on the card goes through here, so the clock, the reading's own
 * timestamp and the reset lines are always written the same way.
 */
export function clockTime(at: Date, format: string, seconds = false): string {
  const twelve = format === "12";
  return at.toLocaleTimeString([], {
    hour: twelve ? "numeric" : "2-digit",
    minute: "2-digit",
    ...(seconds ? { second: "2-digit" as const } : {}),
    hour12: twelve,
  });
}

/** Reset instants are absolute; render them in the viewer's own zone. */
export function resetLabel(iso: string | null, format: string): string {
  if (!iso) return "";
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return "";
  const sameDay = at.toDateString() === new Date().toDateString();
  const time = clockTime(at, format);
  return sameDay
    ? `Resets ${time}`
    : `Resets ${at.toLocaleDateString([], { month: "short", day: "numeric" })}, ${time}`;
}

/**
 * How far through its window a limit is, as a share of the whole.
 *
 * `null` when it cannot be worked out — an unrecognised window, or a reset
 * instant already in the past, would otherwise put the marker somewhere
 * meaningless. The window length comes from the backend, which derives it from
 * the payload rather than assuming a plan.
 */
export function elapsedShare(m: Meter): number | null {
  if (!m.resetsAt || !m.windowSeconds) return null;
  const resets = new Date(m.resetsAt).getTime();
  if (Number.isNaN(resets)) return null;
  const window = m.windowSeconds * 1000;
  const elapsed = (window - (resets - Date.now())) / window;
  return elapsed >= 0 && elapsed <= 1 ? elapsed : null;
}

/** Short phrase for the credit overflow state, or "" when nothing is known. */
export function extraUsagePhrase(extra: ExtraUsage | null): string {
  if (!extra) return "";
  if (extra.enabled) {
    return extra.percent === null
      ? "extra usage on"
      : `extra usage ${Math.round(extra.percent)}%`;
  }
  return extra.disabledReason
    ? `extra usage off (${extra.disabledReason.replace(/_/g, " ")})`
    : "extra usage off";
}

/**
 * Where the figures came from and when.
 *
 * A live read is current. A cached one can be days behind, because Claude Code
 * only rewrites its cache when `/usage` runs, so that line says what to do
 * about it rather than turning red: text wears text colours, and an alarm
 * colour here read as a failure.
 */
/** Why a reading is cached, in the few words the note line has room for. */
export const REASONS: Record<string, string> = {
  expired: "sign-in expired",
  rateLimited: "API is rate-limiting",
  requestFailed: "API did not answer",
  noCredentials: "not signed in here",
};

export function provenance(
  limits: Limits,
  format: string,
): { text: string; detail: string; stale: boolean } {
  const at = new Date(limits.fetchedAt);
  const time = clockTime(at, format);
  const stale = !limits.live && Date.now() - at.getTime() > STALE_MS;
  const when = stale
    ? `${at.toLocaleDateString([], { month: "short", day: "numeric" })} ${time}`
    : time;
  const prefix = limits.live ? "Updated" : "Cached";
  // Say why the live read did not happen. "Run /usage" is only the answer when
  // there is nothing more specific to report.
  const why = limits.reason ? REASONS[limits.reason] : undefined;
  const hint = why ? ` · ${why}` : stale ? " · run /usage to refresh" : "";
  // Source, account and credit state are for when a number looks surprising,
  // which is not often enough to spend a line on.
  const detail = [limits.source, `account ${limits.account}`, extraUsagePhrase(limits.extraUsage)]
    .filter(Boolean)
    .join(" · ");
  return { text: `${prefix} · ${when}${hint}`, detail, stale };
}
