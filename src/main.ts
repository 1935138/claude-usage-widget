// Wiring: the window's state, what it asks of the backend, and the events that
// move it. Everything it draws comes from ./view.

import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { chosen, identity } from "./accounts";
import { clockTime, esc, provenance } from "./format";
import { resolveLang, strings, type Strings } from "./i18n";
import type { ClaudeState, Limits, LoginState, SectionKey, Settings } from "./types";
import { checkForUpdate, installUpdate, type UpdateStatus } from "./updates";
import { render } from "./view/card";
import { type AddingAccount, renderSettings } from "./view/settings";

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

/**
 * Grows the card by a fraction of a pixel so that it ends on a whole device
 * pixel, and answers how tall it then is.
 *
 * The window can only be a whole number of device pixels tall. A card that ends
 * part-way through one leaves the backend to choose: round down and the card's
 * bottom border is clipped off the screen, round up and a pixel of window has
 * no card on it, which - the window being transparent - shows the desktop.
 * Neither is necessary if the card ends where a pixel does.
 */
function snapCardToDevicePixels(): number {
  const dpr = devicePixelRatio || 1;
  cardEl.style.paddingBottom = "0px";
  const measured = cardEl.getBoundingClientRect().height;
  const snapped = Math.ceil(measured * dpr) / dpr;
  if (snapped > measured) cardEl.style.paddingBottom = `${snapped - measured}px`;
  return snapped;
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

  // The card's own box is what the window should match, measured rather than
  // rounded: `offsetHeight` is an integer, and adding up rounded parts left the
  // window a pixel out either way - short of the card clipped its bottom
  // border, over it left a line of desktop showing through.
  //
  // While the icon is up the card is deliberately `100vh`, so measuring it then
  // would only hand the window's own height back; the content it holds is what
  // decides how tall the window should be.
  const style = getComputedStyle(bodyEl);
  const padding = parseFloat(style.paddingTop) + parseFloat(style.paddingBottom);
  const height = loading
    ? contentEl.getBoundingClientRect().height + padding + CARD_BORDERS
    : snapCardToDevicePixels();

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
  extraAccounts: [],
};
/** The words the card is written in, settled once the settings have loaded. */
let t: Strings = strings(resolveLang("system"));
/**
 * Until the first reading is in there is nothing to draw, and a card drawn
 * around nothing says the wrong thing: it would read as "no usage found"
 * rather than "not asked yet". So the window waits, empty and transparent.
 */
let loading = true;
let showingSettings = false;
/** What the last update check found; only ever shown in the settings panel. */
let update: UpdateStatus = { kind: "unknown" };
/** Filled in at startup; the footer shows it while the settings panel is open. */
let version = "";
let accounts: Limits[] = [];
let login: LoginState = "ok";
/** Why the last sign-in attempt never started, if it did not. */
let loginError: string | null = null;
/** The sign-in for an extra account the settings panel started, if any. */
let adding: AddingAccount | null = null;
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
  // Nothing at all until the first reading lands: the card is transparent and
  // stripped while `loading`, so the window is there without showing anything.
  contentEl.innerHTML = loading
    ? ""
    : showingSettings
      ? renderSettings(settings, accounts, adding, t, update)
      : render(accounts, settings, login, loginError, t);
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
    const state: ClaudeState = accounts.length === 0 ? await invoke("login_state") : "ok";
    login = state === "needed" && login === "waiting" ? "waiting" : state;
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
  loginError = null;
  try {
    await invoke("login");
  } catch (err) {
    // Nothing was started, so no console will open and nothing will arrive to
    // poll for. Say why on the card rather than leaving the button looking
    // inert, which is how a missing CLI used to present itself.
    loginError = String(err);
    await draw();
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


/**
 * Signs in one more account, in a config directory of its own.
 *
 * The directory is recorded only once credentials appear in it, so a console
 * closed halfway leaves nothing behind that could never work. What is left on
 * disk is swept the next time the widget starts.
 */
async function startAddAccount(): Promise<void> {
  let dir: string;
  try {
    dir = await invoke<string>("add_account");
  } catch (err) {
    adding = { dir: "", error: String(err) };
    await draw();
    return;
  }
  adding = { dir, error: null };
  await draw();

  let left = LOGIN_POLL_LIMIT;
  const poll = setInterval(() => {
    if (adding === null || adding.dir !== dir) {
      clearInterval(poll);
      return;
    }
    if (left-- <= 0) {
      clearInterval(poll);
      adding = { dir, error: t.addAccountNote };
      void draw();
      return;
    }
    void (async () => {
      if (!(await invoke<boolean>("account_signed_in", { dir }))) return;
      clearInterval(poll);
      try {
        await invoke("confirm_account", { dir });
        settings = await invoke<Settings>("load_settings");
        adding = null;
      } catch (err) {
        adding = { dir, error: String(err) };
      }
      await load();
    })();
  }, LOGIN_POLL_MS);
}

async function removeAccount(dir: string): Promise<void> {
  try {
    await invoke("remove_account", { dir });
    settings = await invoke<Settings>("load_settings");
  } catch {
    return;
  }
  await load();
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
  // Ask once, when the panel is first opened: nobody wants a widget that phones
  // home on a timer to see whether it is out of date.
  if (showingSettings && update.kind === "unknown") void runUpdateCheck();
  void draw();
});
document.getElementById("refresh")!.addEventListener("click", () => void load());
/** Runs a check and redraws the panel around whatever it found. */
async function runUpdateCheck(): Promise<void> {
  update = { kind: "checking" };
  await draw();
  update = await checkForUpdate();
  await draw();
}

contentEl.addEventListener("click", (event) => {
  if (!(event.target instanceof HTMLElement)) return;
  if (event.target.id === "login") {
    void startLogin();
    return;
  }
  if (event.target.id === "install-claude") {
    void invoke("open_install_docs").catch(() => undefined);
    return;
  }
  if (event.target.id === "add-account") {
    void startAddAccount();
    return;
  }
  if (event.target.classList.contains("remove-account")) {
    const dir = event.target.dataset.dir;
    if (dir) void removeAccount(dir);
    return;
  }
  if (event.target.id === "check-update") {
    void runUpdateCheck();
    return;
  }
  if (event.target.id === "install-update") {
    void (async () => {
      update = { kind: "installing" };
      await draw();
      update = await installUpdate();
      await draw();
    })();
  }
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
