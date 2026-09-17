// What the settings panel offers, and the order it offers it in. The words for
// these live in ./i18n; here they are only values.

import type { SectionKey } from "./types";

/**
 * Whether the clock is on the card. The footer's shape is fixed - where the
 * figures came from on the left, the clock on the right - so position is no
 * longer a choice; the stored value stays one of `CLOCK_POSITIONS` so that
 * files written by older versions still read.
 */
export const CLOCK_VALUES = ["right", "off"];

/** How times are written, matching `TIME_FORMATS` in the settings crate. */
export const TIME_FORMAT_VALUES = ["24", "12"];

/** Matching `LANGUAGES` in the settings crate; `system` follows the machine. */
export const LANGUAGE_VALUES = ["system", "en", "ko"];

/**
 * Offered corner radii, in CSS pixels. A hand-edited `settings.json` may name
 * any value up to `MAX_CORNER_RADIUS`; one that is not on this list is added to
 * the menu rather than silently snapped to a neighbour.
 */
export const CORNER_RADII = [0, 6, 12, 20];

/** Offered refresh intervals, in seconds; 0 polls only on demand. */
export const REFRESH_SECONDS = [0, 180, 300, 600, 1800];

/** Identity hues, in the order the limits appear. */
export const HUES = ["var(--cat-1)", "var(--cat-2)", "var(--cat-3)"];

/** Every switchable item, in the order the panel lists them. */
export const SECTION_KEYS: ReadonlyArray<SectionKey> = [
  "session",
  "weekly",
  "pace",
  "provenance",
];
