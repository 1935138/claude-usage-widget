// What the window shows before the first reading lands.

import { esc } from "../format";
import type { Strings } from "../i18n";

/**
 * The identicon, as the 5x5 grid it is rather than the PNG GitHub renders.
 *
 * Inline, and filled with `currentColor`, for the two things a file could not
 * do: the card is transparent while this is up, so the icon has to take the
 * theme's ink or vanish against a dark desktop, and a 5-unit viewBox at a
 * multiple of 5 pixels lands every block on a whole pixel.
 */
const ICON = `<svg class="splash-icon" viewBox="0 0 5 5" width="50" height="50" fill="currentColor" role="img" aria-label="__LABEL__">
      <rect x="1" y="0" width="1" height="1" />
      <rect x="3" y="0" width="1" height="1" />
      <rect x="0" y="1" width="5" height="2" />
      <rect x="0" y="3" width="1" height="1" />
      <rect x="2" y="3" width="1" height="1" />
      <rect x="4" y="3" width="1" height="1" />
      <rect x="0" y="4" width="5" height="1" />
    </svg>`;

export function splash(t: Strings): string {
  return `<div class="splash">${ICON.replace("__LABEL__", esc(t.loading))}</div>`;
}
