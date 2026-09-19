// The shapes that cross the IPC boundary, mirroring the Rust side.

/** Mirrors `claude_usage_core::limits::Meter`. */
export interface Meter {
  kind: string;
  label: string;
  percent: number;
  /** Raised level as the API reports it. Kept for `probe`; the card leaves the
   * bar its own colour and lets the percentage say how full it is. */
  severity: string;
  resetsAt: string | null;
  windowSeconds: number | null;
  isActive: boolean;
  /** Model a scoped limit belongs to; the card builds the label from it. */
  scopeModel: string | null;
}

/** Mirrors `claude_usage_core::limits::ExtraUsage`. */
export interface ExtraUsage {
  enabled: boolean;
  disabledReason: string | null;
  percent: number | null;
}

/** Mirrors `claude_usage_core::limits::Limits`. */
export interface Limits {
  meters: Meter[];
  fetchedAt: string;
  source: string;
  account: string;
  email: string | null;
  organization: string | null;
  plan: string | null;
  live: boolean;
  extraUsage: ExtraUsage | null;
  reason: string | null;
}

/** Mirrors `claude_usage_core::settings::Settings`. */
export interface Settings {
  session: boolean;
  weekly: boolean;
  provenance: boolean;
  pace: boolean;
  account: string | null;
  refreshSeconds: number;
  clock: string;
  timeFormat: string;
  language: string;
  cornerRadius: number;
}

/** Mirrors `commands::ClaudeState`: what the backend found on this machine. */
export type ClaudeState = "ok" | "needed" | "notInstalled";

/**
 * What the card should say about signing in: the backend's answer, plus
 * `waiting` while a sign-in this widget started is still running.
 */
export type LoginState = ClaudeState | "waiting";

/** The boolean switches, as distinct from the account and interval settings. */
export type SectionKey = "session" | "weekly" | "pace" | "provenance";
