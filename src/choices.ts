// What the settings panel offers, and the order it offers it in.

/**
 * Whether the clock is on the card. The footer's shape is fixed - where the
 * figures came from on the left, the clock on the right - so position is no
 * longer a choice; the stored value stays one of `CLOCK_POSITIONS` so that
 * files written by older versions still read.
 */
export const CLOCK_CHOICES: ReadonlyArray<[string, string]> = [
  ["right", "Shown"],
  ["off", "Hidden"],
];

/** How times are written, matching `TIME_FORMATS` in the settings crate. */
export const TIME_FORMAT_CHOICES: ReadonlyArray<[string, string]> = [
  ["24", "24-hour"],
  ["12", "12-hour"],
];

/**
 * Offered corner radii, in CSS pixels. A hand-edited `settings.json` may name
 * any value up to `MAX_CORNER_RADIUS`; one that is not on this list is added to
 * the menu rather than silently snapped to a neighbour.
 */
export const CORNER_CHOICES: ReadonlyArray<[number, string]> = [
  [0, "Square"],
  [6, "Slight"],
  [12, "Rounded"],
  [20, "Very rounded"],
];

/** Offered refresh intervals, in seconds; 0 polls only on demand. */
export const REFRESH_CHOICES: ReadonlyArray<[number, string]> = [
  [0, "Manual only"],
  [180, "3 minutes"],
  [300, "5 minutes"],
  [600, "10 minutes"],
  [1800, "30 minutes"],
];

/** Identity hues, in the order the limits appear. */
export const HUES = ["var(--cat-1)", "var(--cat-2)", "var(--cat-3)"];

/** The boolean switches, as distinct from the account and interval settings. */
export type SectionKey = "session" | "weekly" | "pace" | "provenance";

/** Every switchable item, in the order the panel lists them. */
export const SECTIONS: ReadonlyArray<[SectionKey, string]> = [
  ["session", "Current session"],
  ["weekly", "Weekly limits"],
  ["pace", "Pace marker"],
  ["provenance", "Cache & credits info"],
];
