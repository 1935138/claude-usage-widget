import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Mirrors `claude_usage_core::limits::Meter`. */
interface Meter {
  kind: string;
  label: string;
  percent: number;
  severity: string;
  resetsAt: string | null;
  windowSeconds: number | null;
  isActive: boolean;
}

/** Mirrors `claude_usage_core::limits::ExtraUsage`. */
interface ExtraUsage {
  enabled: boolean;
  disabledReason: string | null;
  percent: number | null;
}

/** Mirrors `claude_usage_core::limits::Limits`. */
interface Limits {
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
interface Settings {
  session: boolean;
  weekly: boolean;
  provenance: boolean;
  pace: boolean;
  account: string | null;
  refreshSeconds: number;
  clock: string;
  cornerRadius: number;
}

/**
 * Whether the clock is on the card. The footer's shape is fixed - where the
 * figures came from on the left, the clock on the right - so position is no
 * longer a choice; the stored value stays one of `CLOCK_POSITIONS` so that
 * files written by older versions still read.
 */
const CLOCK_CHOICES: ReadonlyArray<[string, string]> = [
  ["right", "Shown"],
  ["off", "Hidden"],
];

/**
 * Offered corner radii, in CSS pixels. A hand-edited `settings.json` may name
 * any value up to `MAX_CORNER_RADIUS`; one that is not on this list is added to
 * the menu rather than silently snapped to a neighbour.
 */
const CORNER_CHOICES: ReadonlyArray<[number, string]> = [
  [0, "Square"],
  [6, "Slight"],
  [12, "Rounded"],
  [20, "Very rounded"],
];

/** Offered refresh intervals, in seconds; 0 polls only on demand. */
const REFRESH_CHOICES: ReadonlyArray<[number, string]> = [
  [0, "Manual only"],
  [180, "3 minutes"],
  [300, "5 minutes"],
  [600, "10 minutes"],
  [1800, "30 minutes"],
];
/** A cache older than this is called out; it only refreshes when Claude Code runs. */
const STALE_MS = 60 * 60_000;
/** The card's 1px top and bottom border, which the content measurement misses. */
const CARD_BORDERS = 2;
/** Ignore sub-pixel differences so a refresh does not jiggle the window. */
const FIT_EPSILON = 2;

const esc = (s: string): string =>
  s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

/** Reset instants are absolute; render them in the viewer's own zone. */
function resetLabel(iso: string | null): string {
  if (!iso) return "";
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return "";
  const sameDay = at.toDateString() === new Date().toDateString();
  const time = at.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  return sameDay
    ? `Resets ${time}`
    : `Resets ${at.toLocaleDateString([], { month: "short", day: "numeric" })}, ${time}`;
}

/**
 * How far through its window a limit is, as a share of the whole.
 *
 * `null` when it cannot be worked out — an unrecognised window, or a reset
 * instant already in the past, would otherwise put the marker somewhere
 * meaningless. The window length comes from the backend, which derives it from
 * the payload rather than assuming a plan.
 */
function elapsedShare(m: Meter): number | null {
  if (!m.resetsAt || !m.windowSeconds) return null;
  const resets = new Date(m.resetsAt).getTime();
  if (Number.isNaN(resets)) return null;
  const window = m.windowSeconds * 1000;
  const elapsed = (window - (resets - Date.now())) / window;
  return elapsed >= 0 && elapsed <= 1 ? elapsed : null;
}

/**
 * The pace marker: where the clock has got to, against where the bar has got
 * to. A bar ahead of its marker is burning the window faster than the window
 * is passing, and will run out before the reset.
 */
function paceMarker(m: Meter, show: boolean): string {
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

/** Status names the palette defines; anything else is treated as normal. */
const STATUSES = ["warning", "serious", "critical"];
/** Above these shares of a limit, the bar stops being merely informative. */
const WARNING_AT = 70;
const CRITICAL_AT = 90;

/**
 * How alarming a meter is: what the API says if it has raised anything, else
 * how close the bar has crept to the cap.
 */
function level(m: Meter): string {
  if (STATUSES.includes(m.severity)) return m.severity;
  if (m.percent >= CRITICAL_AT) return "critical";
  if (m.percent >= WARNING_AT) return "warning";
  return "normal";
}

/**
 * Fill colour: normally the limit's own identity hue, but a status colour once
 * the bar runs hot. Status colours are reserved for exactly this, so they never
 * read as just another series.
 */
function fillColour(m: Meter, hue: string): string {
  const state = level(m);
  return state === "normal" ? hue : `var(--status-${state})`;
}

/**
 * A meter: one ratio against a plan limit. The percentage is always written
 * out, and a raised state is spelled out in the badge, so colour is never the
 * only carrier.
 */
function meter(m: Meter, previousReset: string, hue: string, pace: boolean): string {
  const pct = Math.max(0, Math.min(100, m.percent));
  const state = level(m);
  const severity = state !== "normal" ? state : "";
  const reset = resetLabel(m.resetsAt);
  // The weekly meters share a reset instant; printing it twice is noise.
  const showReset = reset && reset !== previousReset ? reset : "";
  return `<div class="meter${m.isActive ? " is-active" : ""}">
      <div class="row-head">
        <span class="row-label">${esc(m.label)}</span>
        ${severity ? `<span class="badge">${esc(severity)}</span>` : ""}
        <span class="row-value">${Math.round(m.percent)}%</span>
      </div>
      <div class="track">
        <div class="fill" style="width:${pct}%;background:${esc(fillColour(m, hue))}"></div>
        ${paceMarker(m, pace)}
      </div>
      ${showReset ? `<div class="meter-reset">${esc(showReset)}</div>` : ""}
    </div>`;
}

/** Identity hues, in the order the limits appear. */
const HUES = ["var(--cat-1)", "var(--cat-2)", "var(--cat-3)"];

/**
 * Colour follows the kind of limit, not its position, so adding or hiding a
 * meter never repaints the others. Several per-model limits take the remaining
 * slots in turn.
 */
function hueFor(m: Meter, scopedSeen: number): string {
  if (m.kind === "session") return HUES[0]!;
  if (m.kind === "weekly_all") return HUES[1]!;
  return HUES[2 + (scopedSeen % (HUES.length - 2))]!;
}

/** Whether a meter is wanted, given the section switches. */
const wanted = (m: Meter, s: Settings): boolean =>
  m.kind === "session" ? s.session : s.weekly;

/** The account whose meters are shown: the chosen one, else the freshest. */
/**
 * What makes two readings the same account, matching the backend's own rule.
 *
 * The short account id is not it: one login can be recorded under a different
 * `account_uuid` by each install, so keying on it loses track of an account the
 * moment a different install's reading wins.
 */
const identity = (l: Limits): string => l.email ?? l.account;

function chosen(all: Limits[], s: Settings): Limits | undefined {
  // `account` held a short account id before it held an email; a stored one of
  // either kind still picks its account out.
  return (
    all.find((l) => identity(l) === s.account) ?? all.find((l) => l.account === s.account) ?? all[0]
  );
}

/** The email, or the account id for a cache that recorded none. */
const accountName = (l: Limits): string => l.email ?? l.account;

/**
 * Picker plus the selected account's plan and organization.
 *
 * The dropdown only appears with more than one account, but the plan line is
 * worth showing either way.
 */
function accountPicker(all: Limits[], current: Limits): string {
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

/** Short phrase for the credit overflow state, or "" when nothing is known. */
function extraUsagePhrase(extra: ExtraUsage | null): string {
  if (!extra) return "";
  if (extra.enabled) {
    return extra.percent === null
      ? "extra usage on"
      : `extra usage ${Math.round(extra.percent)}%`;
  }
  return extra.disabledReason
    ? `extra usage off (${extra.disabledReason.replace(/_/g, " ")})`
    : "extra usage off";
}

/**
 * Where the figures came from and when.
 *
 * A live read is current. A cached one can be days behind, because Claude Code
 * only rewrites its cache when `/usage` runs, so that line says what to do
 * about it rather than turning red: text wears text colours, and an alarm
 * colour here read as a failure.
 */
/** Why a reading is cached, in the few words the note line has room for. */
const REASONS: Record<string, string> = {
  expired: "sign-in expired",
  rateLimited: "API is rate-limiting",
  requestFailed: "API did not answer",
  noCredentials: "not signed in here",
};

function provenance(limits: Limits): { text: string; detail: string; stale: boolean } {
  const at = new Date(limits.fetchedAt);
  const time = at.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  const stale = !limits.live && Date.now() - at.getTime() > STALE_MS;
  const when = stale
    ? `${at.toLocaleDateString([], { month: "short", day: "numeric" })} ${time}`
    : time;
  const prefix = limits.live ? "Live" : "Cached";
  // Say why the live read did not happen. "Run /usage" is only the answer when
  // there is nothing more specific to report.
  const why = limits.reason ? REASONS[limits.reason] : undefined;
  const hint = why ? ` · ${why}` : stale ? " · run /usage to refresh" : "";
  // Source, account and credit state are for when a number looks surprising,
  // which is not often enough to spend a line on.
  const detail = [limits.source, `account ${limits.account}`, extraUsagePhrase(limits.extraUsage)]
    .filter(Boolean)
    .join(" · ");
  return { text: `${prefix} · ${when}${hint}`, detail, stale };
}

/**
 * Whether this machine has a Claude Code login: `needed` when none was found,
 * `waiting` while a `claude login` this widget started is still running.
 */
type LoginState = "ok" | "needed" | "waiting";

/** Centred prompt shown when this machine has never run `claude login`. */
function loginRequired(state: LoginState): string {
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

function render(all: Limits[], s: Settings, login: LoginState): string {
  if (all.length === 0) {
    return login === "ok"
      ? `<p class="note">No cached usage found. Run Claude Code once to populate it.</p>`
      : loginRequired(login);
  }
  const limits = chosen(all, s)!;
  const meters = limits.meters.filter((m) => wanted(m, s));

  let previousReset = "";
  let scopedSeen = 0;
  const bars = meters
    .map((m) => {
      const hue = hueFor(m, scopedSeen);
      if (m.kind !== "session" && m.kind !== "weekly_all") scopedSeen += 1;
      const html = meter(m, previousReset, hue, s.pace);
      previousReset = resetLabel(m.resetsAt) || previousReset;
      return html;
    })
    .join("");

  return `
    ${accountPicker(all, limits)}
    ${bars || `<p class="note">Nothing selected. Use the sliders to turn a meter back on.</p>`}`;
}

/** The boolean switches, as distinct from the account and interval settings. */
type SectionKey = "session" | "weekly" | "pace" | "provenance";

/** Every switchable item, in the order the panel lists them. */
const SECTIONS: ReadonlyArray<[SectionKey, string]> = [
  ["session", "Current session"],
  ["weekly", "Weekly limits"],
  ["pace", "Pace marker"],
  ["provenance", "Cache & credits info"],
];

function renderSettings(s: Settings): string {
  const rows = SECTIONS.map(
    ([key, label]) => `<label class="opt">
      <input type="checkbox" data-key="${esc(key)}"${s[key] ? " checked" : ""} />
      <span>${esc(label)}</span>
    </label>`,
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
        <span>Corners</span>
        <select id="corner-radius" title="How rounded the card's corners are; the window has no frame of its own">${corners}</select>
      </div>
    </section>`;
}

const bodyEl = document.getElementById("body")!;
const contentEl = document.getElementById("content")!;
const footerEl = document.getElementById("footer")!;
const noteEl = document.getElementById("provenance")!;
const clockEl = document.getElementById("clock")!;

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
 * right. The row goes altogether when it would carry neither.
 */
function applyFooter(): void {
  const limits = showingSettings ? undefined : chosen(accounts, settings);
  const note = limits && settings.provenance ? provenance(limits) : undefined;
  noteEl.textContent = note?.text ?? "";
  noteEl.title = note?.detail ?? "";
  noteEl.classList.toggle("stale", note?.stale ?? false);

  clockEl.hidden = settings.clock === "off";
  footerEl.hidden = clockEl.hidden && !note;
}

function tick(): void {
  clockEl.textContent = new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
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
  cornerRadius: 12,
};
let showingSettings = false;
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
  contentEl.innerHTML = showingSettings
    ? renderSettings(settings)
    : render(accounts, settings, login);
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
    contentEl.innerHTML = `<p class="note">Could not read usage: ${esc(String(err))}</p>`;
    await fitWindow();
    return;
  }
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
  applyCornerRadius();
  applyFooter();
  scheduleRefresh();
  await load();
})();
