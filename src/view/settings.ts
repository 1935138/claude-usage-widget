// The panel behind the sliders icon.

import { accountName } from "../accounts";
import {
  CLOCK_VALUES,
  CORNER_RADII,
  LANGUAGE_VALUES,
  MAX_EXTRA_ACCOUNTS,
  REFRESH_SECONDS,
  SECTION_KEYS,
  TIME_FORMAT_VALUES,
} from "../choices";
import { esc } from "../format";
import type { Strings } from "../i18n";
import type { Limits, Settings } from "../types";
import type { UpdateStatus } from "../updates";

/** One `<option>` per value, with the catalogue supplying the words. */
function options<T extends string | number>(
  values: readonly T[],
  selected: T,
  label: (value: T) => string,
): string {
  return values
    .map(
      (value) =>
        `<option value="${esc(String(value))}"${value === selected ? " selected" : ""}>${esc(
          label(value),
        )}</option>`,
    )
    .join("");
}

/** A row of the panel: what it sets on the left, the control on the right. */
function row(label: string, control: string): string {
  return `<div class="opt-row">
        <span>${esc(label)}</span>
        ${control}
      </div>`;
}

/** The update row's control: a button where there is something to do, else a word. */
function updateControl(status: UpdateStatus, t: Strings): string {
  switch (status.kind) {
    case "checking":
      return `<span class="opt-note">${esc(t.updateChecking)}</span>`;
    case "current":
      // Still a button: a widget that has said "up to date" once should not
      // have to be restarted to say it again an hour later.
      return `<button id="check-update" class="opt-btn" type="button" title="${esc(
        t.updateCheck,
      )}">${esc(t.updateCurrent)}</button>`;
    case "available":
      return `<button id="install-update" class="opt-btn" type="button">${esc(
        t.updateAvailable(status.version),
      )}</button>`;
    case "installing":
      return `<span class="opt-note">${esc(t.updateInstalling)}</span>`;
    case "failed":
      return `<button id="check-update" class="opt-btn" type="button">${esc(t.updateFailed)}</button>`;
    default:
      return `<button id="check-update" class="opt-btn" type="button">${esc(t.updateCheck)}</button>`;
  }
}


/** Whether a sign-in the panel started is still running, and where. */
export interface AddingAccount {
  dir: string;
  error: string | null;
}

/**
 * The accounts the card can show, and the button that signs in another.
 *
 * Every account is listed, not only the ones the widget added, because the
 * ones it did not add are exactly what makes the list confusing otherwise: an
 * install found under WSL has no remove button, and saying where it came from
 * is what explains why.
 */
function accountsSection(
  all: Limits[],
  s: Settings,
  adding: AddingAccount | null,
  t: Strings,
): string {
  const rows = all
    .map((one) => {
      const removable = one.configDir !== "" && s.extraAccounts.includes(one.configDir);
      const button = removable
        ? `<button class="opt-btn remove-account" type="button" data-dir="${esc(one.configDir)}">${esc(
            t.removeAccount,
          )}</button>`
        : "";
      // A directory the widget made is named by a path nobody chose to read;
      // where it came from is the useful part, and the path stays on hover.
      const from = removable ? t.accountAdded : t.accountSource(one.source);
      return `<div class="opt-row account-row">
        <span class="account-line">
          <span class="account-id">${esc(accountName(one))}</span>
          <span class="account-from" title="${esc(one.configDir || one.source)}">${esc(from)}</span>
        </span>
        ${button}
      </div>`;
    })
    .join("");

  const full = s.extraAccounts.length >= MAX_EXTRA_ACCOUNTS;
  const waiting = adding !== null && adding.error === null;
  const control = waiting
    ? `<button id="add-account" class="opt-btn" type="button" disabled>${esc(t.addAccountWaiting)}</button>`
    : `<button id="add-account" class="opt-btn" type="button"${full ? " disabled" : ""}>${esc(
        t.addAccount,
      )}</button>`;

  const note = waiting
    ? `<p class="note">${esc(t.addAccountNote)}</p>`
    : adding?.error
      ? `<p class="note">${esc(t.addAccountFailed(adding.error))}</p>`
      : full
        ? `<p class="note">${esc(t.accountsFull)}</p>`
        : "";

  return `<section class="section" title="${esc(t.accountsTitle)}">
      <span class="section-title">${esc(t.accounts)}</span>
      ${rows}
      ${row("", control)}
      ${note}
    </section>`;
}

export function renderSettings(
  s: Settings,
  accounts: Limits[],
  adding: AddingAccount | null,
  t: Strings,
  update: UpdateStatus,
): string {
  const switches = SECTION_KEYS.map(
    (key) => `<label class="opt">
      <input type="checkbox" data-key="${esc(key)}"${s[key] ? " checked" : ""} />
      <span>${esc(t.section(key))}</span>
    </label>`,
  ).join("");

  // A hand-edited radius the panel does not offer joins the menu rather than
  // being silently snapped to a neighbour.
  const radii = CORNER_RADII.includes(s.cornerRadius)
    ? CORNER_RADII
    : [...CORNER_RADII, s.cornerRadius].sort((a, b) => a - b);

  return `${accountsSection(accounts, s, adding, t)}
    <section class="section">
      <span class="section-title">${esc(t.show)}</span>
      ${switches}
      ${row(
        t.refreshEvery,
        `<select id="interval" title="${esc(t.refreshTitle)}">${options(
          REFRESH_SECONDS,
          s.refreshSeconds,
          t.refreshChoice,
        )}</select>`,
      )}
      ${row(t.clock, `<select id="clock-position">${options(CLOCK_VALUES, s.clock, t.clockChoice)}</select>`)}
      ${row(
        t.timeFormat,
        `<select id="time-format" title="${esc(t.timeFormatTitle)}">${options(
          TIME_FORMAT_VALUES,
          s.timeFormat,
          t.timeFormatChoice,
        )}</select>`,
      )}
      ${row(
        t.language,
        `<select id="language">${options(LANGUAGE_VALUES, s.language, t.languageChoice)}</select>`,
      )}
      ${row(t.update, updateControl(update, t))}
      ${row(
        t.corners,
        `<select id="corner-radius" title="${esc(t.cornersTitle)}">${options(
          radii,
          s.cornerRadius,
          t.cornerChoice,
        )}</select>`,
      )}
    </section>`;
}
