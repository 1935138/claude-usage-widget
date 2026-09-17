// The panel behind the sliders icon.

import {
  CLOCK_CHOICES,
  CORNER_CHOICES,
  REFRESH_CHOICES,
  SECTIONS,
  TIME_FORMAT_CHOICES,
} from "../choices";
import { esc } from "../format";
import type { Settings } from "../types";

export function renderSettings(s: Settings): string {
  const rows = SECTIONS.map(
    ([key, label]) => `<label class="opt">
      <input type="checkbox" data-key="${esc(key)}"${s[key] ? " checked" : ""} />
      <span>${esc(label)}</span>
    </label>`,
  ).join("");
  const formats = TIME_FORMAT_CHOICES.map(
    ([value, label]) =>
      `<option value="${esc(value)}"${value === s.timeFormat ? " selected" : ""}>${esc(label)}</option>`,
  ).join("");
  const clocks = CLOCK_CHOICES.map(
    ([value, label]) =>
      `<option value="${esc(value)}"${value === s.clock ? " selected" : ""}>${esc(label)}</option>`,
  ).join("");
  const custom: [number, string] = [s.cornerRadius, `${s.cornerRadius}px`];
  const radii: ReadonlyArray<[number, string]> = CORNER_CHOICES.some(
    ([px]) => px === s.cornerRadius,
  )
    ? CORNER_CHOICES
    : [...CORNER_CHOICES, custom].sort((a, b) => a[0] - b[0]);
  const corners = radii
    .map(
      ([px, label]) =>
        `<option value="${px}"${px === s.cornerRadius ? " selected" : ""}>${esc(label)}</option>`,
    )
    .join("");
  const options = REFRESH_CHOICES.map(
    ([seconds, label]) =>
      `<option value="${seconds}"${seconds === s.refreshSeconds ? " selected" : ""}>${esc(label)}</option>`,
  ).join("");
  return `<section class="section">
      <span class="section-title">Show</span>
      ${rows}
      <div class="opt-row">
        <span>Refresh every</span>
        <select id="interval" title="Each refresh is one request to the usage API">${options}</select>
      </div>
      <div class="opt-row">
        <span>Clock</span>
        <select id="clock-position">${clocks}</select>
      </div>
      <div class="opt-row">
        <span>Time format</span>
        <select id="time-format" title="Applies to the clock, the reading's time and the reset lines">${formats}</select>
      </div>
      <div class="opt-row">
        <span>Corners</span>
        <select id="corner-radius" title="How rounded the card's corners are; the window has no frame of its own">${corners}</select>
      </div>
    </section>`;
}
