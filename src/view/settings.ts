// The panel behind the sliders icon.

import {
  CLOCK_VALUES,
  CORNER_RADII,
  LANGUAGE_VALUES,
  REFRESH_SECONDS,
  SECTION_KEYS,
  TIME_FORMAT_VALUES,
} from "../choices";
import { esc } from "../format";
import type { Strings } from "../i18n";
import type { Settings } from "../types";
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
      return `<span class="opt-note">${esc(t.updateCurrent)}</span>`;
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

export function renderSettings(s: Settings, t: Strings, update: UpdateStatus): string {
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

  return `<section class="section">
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
