// The card itself: the account line, a bar per limit, and what stands in for
// them when there is nothing to show.

import { accountName, chosen, identity } from "../accounts";
import { HUES } from "../choices";
import { elapsedShare, esc, resetLabel } from "../format";
import type { Limits, LoginState, Meter, Settings } from "../types";

/**
 * The pace marker: where the clock has got to, against where the bar has got
 * to. A bar ahead of its marker is burning the window faster than the window
 * is passing, and will run out before the reset.
 */
export function paceMarker(m: Meter, show: boolean): string {
  if (!show) return "";
  const share = elapsedShare(m);
  if (share === null) return "";
  const elapsed = Math.round(share * 100);
  const verdict =
    m.percent > elapsed + 1
      ? `using faster than the clock (${Math.round(m.percent)}% used, ${elapsed}% elapsed)`
      : `within pace (${Math.round(m.percent)}% used, ${elapsed}% elapsed)`;
  return `<div class="pace" style="left:${share * 100}%" title="${esc(verdict)}"></div>`;
}

/**
 * A meter: one ratio against a plan limit.
 *
 * Colour is identity only - each limit keeps its own hue however full it is.
 * How close a bar has crept to its cap is carried by the percentage beside it
 * and by the pace marker, not by turning the bar amber or red.
 */
export function meter(m: Meter, hue: string, pace: boolean, format: string): string {
  const pct = Math.max(0, Math.min(100, m.percent));
  const reset = resetLabel(m.resetsAt, format);
  return `<div class="meter${m.isActive ? " is-active" : ""}">
      <div class="row-head">
        <span class="row-label">${esc(m.label)}</span>
        <span class="row-value">${Math.round(m.percent)}%</span>
      </div>
      <div class="track">
        <div class="fill" style="width:${pct}%;background:${esc(hue)}"></div>
        ${paceMarker(m, pace)}
      </div>
      ${reset ? `<div class="meter-reset">${esc(reset)}</div>` : ""}
    </div>`;
}

/**
 * Colour follows the kind of limit, not its position, so adding or hiding a
 * meter never repaints the others. Several per-model limits take the remaining
 * slots in turn.
 */
export function hueFor(m: Meter, scopedSeen: number): string {
  if (m.kind === "session") return HUES[0]!;
  if (m.kind === "weekly_all") return HUES[1]!;
  return HUES[2 + (scopedSeen % (HUES.length - 2))]!;
}

/** Whether a meter is wanted, given the section switches. */
export const wanted = (m: Meter, s: Settings): boolean =>
  m.kind === "session" ? s.session : s.weekly;

/**
 * Picker plus the selected account's plan and organization.
 *
 * The dropdown only appears with more than one account, but the plan line is
 * worth showing either way.
 */
export function accountPicker(all: Limits[], current: Limits): string {
  const picker =
    all.length < 2
      ? `<div class="account-name">${esc(accountName(current))}</div>`
      : `<select id="account" class="picker" title="Quota is per account; these installs are signed in as different ones">${all
          .map(
            (l) =>
              `<option value="${esc(identity(l))}"${identity(l) === identity(current) ? " selected" : ""}>${esc(accountName(l))}${l.live ? "" : " (cached)"}</option>`,
          )
          .join("")}</select>`;

  const meta = [current.plan, current.organization].filter(Boolean).join(" · ");
  return `<div class="account">
      ${picker}
      ${meta ? `<div class="account-meta">${esc(meta)}</div>` : ""}
    </div>`;
}

/** Centred prompt shown when this machine has never run `claude login`. */
export function loginRequired(state: LoginState): string {
  const waiting = state === "waiting";
  const note = waiting
    ? "Complete the sign-in in the console window."
    : "No Claude Code login found on this machine.";
  return `<div class="login-panel">
      <p class="note">${note}</p>
      <button id="login" class="login-btn" type="button"${waiting ? " disabled" : ""}>${
        waiting ? "Waiting for sign-in…" : "Login Required"
      }</button>
    </div>`;
}

export function render(all: Limits[], s: Settings, login: LoginState): string {
  if (all.length === 0) {
    return login === "ok"
      ? `<p class="note">No cached usage found. Run Claude Code once to populate it.</p>`
      : loginRequired(login);
  }
  const limits = chosen(all, s)!;
  const meters = limits.meters.filter((m) => wanted(m, s));

  let scopedSeen = 0;
  const bars = meters
    .map((m) => {
      const hue = hueFor(m, scopedSeen);
      if (m.kind !== "session" && m.kind !== "weekly_all") scopedSeen += 1;
      return meter(m, hue, s.pace, s.timeFormat);
    })
    .join("");

  return `
    ${accountPicker(all, limits)}
    ${bars || `<p class="note">Nothing selected. Use the sliders to turn a meter back on.</p>`}`;
}
