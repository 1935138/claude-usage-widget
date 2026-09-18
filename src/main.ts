// Wiring: the window's state, what it asks of the backend, and the events that
// move it. Everything it draws comes from ./view.

import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { chosen, identity } from "./accounts";
import { clockTime, esc, provenance } from "./format";
import { resolveLang, strings, type Strings } from "./i18n";
import type { Limits, LoginState, SectionKey, Settings } from "./types";
import { render } from "./view/card";
import { renderSettings } from "./view/settings";
import { splash } from "./view/splash";

/** The card's 1px top and bottom border, which the content measurement misses. */
const CARD_BORDERS = 2;
/** Ignore sub-pixel differences so a refresh does not jiggle the window. */
const FIT_EPSILON = 2;

const cardEl = document.getElementById("card")!;
const bodyEl = document.getElementById("body")!;
const contentEl = document.getElementById("content")!;
const footerEl = document.getElementById("footer")!;
const noteEl = document.getElementById("provenance")!;
const versionEl = document.getElementById("version")!;
const clockEl = document.getElementById("clock")!;

/**
 * Applies the chosen language.
 *
 * The title bar is the only text outside the rendered card, so it is set from
 * here rather than carried in the markup.
 */
function applyLanguage(): void {
  t = strings(resolveLang(settings.language));
  document.getElementById("settings")!.title = t.settings;
  document.getElementById("refresh")!.title = t.refresh;
  document.getElementById("close")!.title = t.close;
}

/**
 * Applies the card's corner radius.
 *
 * Set as a custom property rather than on the element, so the stylesheet keeps
 * the whole of the card's shape in one place.
 */
function applyCornerRadius(): void {
  document.documentElement.style.setProperty("--corner-radius", `${settings.cornerRadius}px`);
}

/**
 * Fills the footer: where the figures came from on the left, the clock on the
 * right, and, while the settings panel is open, the version in the middle -
 * the panel has no reading to report, and the row would otherwise be empty or
 * gone. The row goes altogether only when it would carry nothing at all.
 */
function applyFooter(): void {
  const limits = showingSettings ? undefined : chosen(accounts, settings);
  const note =
    limits && settings.provenance ? provenance(limits, settings.timeFormat, t) : undefined;
  noteEl.textContent = note?.text ?? "";
  noteEl.title = note?.detail ?? "";
  noteEl.classList.toggle("stale", note?.stale ?? false);

  versionEl.textContent = showingSettings ? version : "";
  clockEl.hidden = settings.clock === "off";
  footerEl.hidden = clockEl.hidden && !note && !versionEl.textContent;
}

function tick(): void {
  clockEl.textContent = clockTime(new Date(), settings.timeFormat, t, true);
}

/** Last height handed to the backend, to avoid resizing on every refresh. */
let lastFitted = 0;

/**
 * Measures how tall the widget actually rendered and asks the backend to make
 * the window that tall. The window is undecorated and fixed-size, so this is
 * what keeps every row visible instead of hidden behind a scrollbar; the
 * clamping against the display's work area happens on the Rust side.
 */
async function fitWindow(): Promise<void> {
  // Web fonts change text metrics, and layout has to have been applied, or the
  // first measurement comes out short.
  await document.fonts.ready.catch(() => undefined);
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

  const bar = document.querySelector<HTMLElement>(".bar");
  const style = getComputedStyle(bodyEl);
  const padding = parseFloat(style.paddingTop) + parseFloat(style.paddingBottom);
  // `contentEl` keeps its natural height; `bodyEl` is flex:1 and would just
  // report the viewport back.
  const height =
    (bar?.offsetHeight ?? 0) +
    contentEl.offsetHeight +
    padding +
    footerEl.offsetHeight +
    CARD_BORDERS;

  if (!Number.isFinite(height) || Math.abs(height - lastFitted) < FIT_EPSILON) {
    return;
  }
  lastFitted = height;
  try {
    await invoke("fit_to_content", { height });
  } catch {
    lastFitted = 0; // Let the next refresh try again.
  }
}

/** Defaults stand in until the backend answers, and if it never does. */
let settings: Settings = {
  session: true,
  weekly: true,
  provenance: true,
  pace: true,
  account: null,
  refreshSeconds: 300,
  clock: "right",
  timeFormat: "24",
  language: "system",
  cornerRadius: 12,
};
/** The words the card is written in, settled once the settings have loaded. */
let t: Strings = strings(resolveLang("system"));
/**
 * Until the first reading is in there is nothing to draw, and a card drawn
 * around nothing says the wrong thing: it would read as "no usage found"
 * rather than "not asked yet".
 */
let loading = true;
let showingSettings = false;
/** Filled in at startup; the footer shows it while the settings panel is open. */
let version = "";
let accounts: Limits[] = [];
let login: LoginState = "ok";
/**
 * The last live answer per account.
 *
 * A failed fetch falls back to Claude Code's cache, which can be weeks behind,
 * and letting that replace a live reading made the figures flip between two
 * very different numbers. Once an account has answered live, it keeps showing
 * live values.
 */
const lastLive = new Map<string, Limits>();

async function draw(): Promise<void> {
  cardEl.classList.toggle("loading", loading);
  contentEl.innerHTML = loading
    ? splash(t)
    : showingSettings
      ? renderSettings(settings, t)
      : render(accounts, settings, login, t);
  applyFooter();
  await fitWindow();
}

function persist(): void {
  void invoke("save_settings", { settings }).catch(() => undefined);
}

let timer: number | undefined;

/** (Re)arms the poll, or leaves it off when the interval is "manual only". */
function scheduleRefresh(): void {
  if (timer !== undefined) clearInterval(timer);
  timer =
    settings.refreshSeconds > 0
      ? setInterval(() => void load(), settings.refreshSeconds * 1000)
      : undefined;
}

async function load(): Promise<void> {
  try {
    const fetched = await invoke<Limits[]>("plan_limits");
    for (const one of fetched) {
      if (one.live) lastLive.set(identity(one), one);
    }
    accounts = fetched.map((one) => (one.live ? one : (lastLive.get(identity(one)) ?? one)));
    // Only worth asking when there is nothing else to show. A sign-in already
    // under way keeps its state until it lands, so the prompt does not flip
    // back while the console window is still open.
    const missing = accounts.length === 0 && (await invoke<boolean>("needs_login"));
    login = missing ? (login === "waiting" ? "waiting" : "needed") : "ok";
  } catch (err) {
    loading = false;
    cardEl.classList.remove("loading");
    contentEl.innerHTML = `<p class="note">${esc(t.readError(String(err)))}</p>`;
    await fitWindow();
    return;
  }
  loading = false;
  await draw();
}

/** How long to keep watching for a sign-in to land, and how often to look. */
const LOGIN_POLL_MS = 3000;
const LOGIN_POLL_LIMIT = 60;

/**
 * Starts `claude login` and watches for it to finish.
 *
 * The sign-in runs in a console of its own, so nothing tells the widget when it
 * lands - it is polled for a few minutes rather than left until the next
 * refresh, which can be five minutes away.
 */
async function startLogin(): Promise<void> {
  try {
    await invoke("login");
  } catch {
    return;
  }
  login = "waiting";
  await draw();
  let left = LOGIN_POLL_LIMIT;
  const poll = setInterval(() => {
    if (login !== "waiting") {
      clearInterval(poll);
      return;
    }
    if (left-- <= 0) {
      clearInterval(poll);
      login = "needed";
      void draw();
      return;
    }
    void load();
  }, LOGIN_POLL_MS);
}

contentEl.addEventListener("change", (event) => {
  const target = event.target;
  if (target instanceof HTMLSelectElement && target.id === "account") {
    settings.account = target.value;
    persist();
    void draw();
    return;
  }
  if (target instanceof HTMLSelectElement && target.id === "interval") {
    settings.refreshSeconds = Number(target.value);
    persist();
    scheduleRefresh();
    return;
  }
  if (target instanceof HTMLSelectElement && target.id === "corner-radius") {
    settings.cornerRadius = Number(target.value);
    persist();
    applyCornerRadius();
    return;
  }
  if (target instanceof HTMLSelectElement && target.id === "time-format") {
    settings.timeFormat = target.value;
    persist();
    tick();
    // Every time on the card is written by this setting, so redraw the lot.
    void draw();
    return;
  }
  if (target instanceof HTMLSelectElement && target.id === "language") {
    settings.language = target.value;
    persist();
    applyLanguage();
    void draw();
    return;
  }
  if (target instanceof HTMLSelectElement && target.id === "clock-position") {
    settings.clock = target.value;
    persist();
    applyFooter();
    // Showing or hiding the footer changes how tall the card is.
    void fitWindow();
    return;
  }
  if (!(target instanceof HTMLInputElement) || !target.dataset.key) return;
  const key = target.dataset.key as SectionKey;
  if (key in settings) {
    settings[key] = target.checked;
    persist();
    void draw();
  }
});

document.getElementById("settings")!.addEventListener("click", () => {
  showingSettings = !showingSettings;
  document.getElementById("settings")!.classList.toggle("on", showingSettings);
  void draw();
});
document.getElementById("refresh")!.addEventListener("click", () => void load());
contentEl.addEventListener("click", (event) => {
  if (!(event.target instanceof HTMLElement) || event.target.id !== "login") return;
  void startLogin();
});
document.getElementById("close")!.addEventListener("click", () => void getCurrentWindow().close());

tick();
setInterval(tick, 1000);

void (async () => {
  try {
    settings = await invoke<Settings>("load_settings");
  } catch {
    // Keep the defaults; everything stays visible.
  }
  version = await getVersion()
    .then((v) => `v${v}`)
    .catch(() => "");
  applyLanguage();
  applyCornerRadius();
  applyFooter();
  // Draw before reading anything: this is what asks the backend to show the
  // window, and a live read can take seconds.
  await draw();
  scheduleRefresh();
  await load();
})();
