import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Mirrors `claude_usage_core::limits::Meter`. */
interface Meter {
  kind: string;
  label: string;
  percent: number;
  severity: string;
  resetsAt: string | null;
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
}

/** Mirrors `claude_usage_core::settings::Settings`. */
interface Settings {
  session: boolean;
  weekly: boolean;
  provenance: boolean;
  account: string | null;
  refreshSeconds: number;
  clock: string;
}

/** Where the clock sits, matching `CLOCK_POSITIONS` in the settings crate. */
const CLOCK_CHOICES: ReadonlyArray<[string, string]> = [
  ["off", "Hidden"],
  ["left", "Bottom left"],
  ["center", "Bottom centre"],
  ["right", "Bottom right"],
];

/** Offered refresh intervals, in seconds; 0 polls only on demand. */
const REFRESH_CHOICES: ReadonlyArray<[number, string]> = [
  [0, "Manual only"],
  [30, "30 seconds"],
  [60, "1 minute"],
  [120, "2 minutes"],
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
function meter(m: Meter, previousReset: string, hue: string): string {
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
      <div class="track"><div class="fill" style="width:${pct}%;background:${esc(fillColour(m, hue))}"></div></div>
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
function chosen(all: Limits[], s: Settings): Limits | undefined {
  return all.find((l) => l.account === s.account) ?? all[0];
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
              `<option value="${esc(l.account)}"${l.account === current.account ? " selected" : ""}>${esc(accountName(l))}</option>`,
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
function provenance(limits: Limits): string {
  const at = new Date(limits.fetchedAt);
  const time = at.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  const stale = !limits.live && Date.now() - at.getTime() > STALE_MS;
  const when = stale
    ? `${at.toLocaleDateString([], { month: "short", day: "numeric" })} ${time}`
    : time;
  const prefix = limits.live ? "Live" : "Cached";
  const hint = stale ? " · run /usage to refresh" : "";
  // Source, account and credit state are for when a number looks surprising,
  // which is not often enough to spend a line on.
  const detail = [limits.source, `account ${limits.account}`, extraUsagePhrase(limits.extraUsage)]
    .filter(Boolean)
    .join(" · ");
  return `<p class="note${stale ? " stale" : ""}" title="${esc(detail)}">${esc(prefix)} · ${esc(when)}${esc(hint)}</p>`;
}

function render(all: Limits[], s: Settings): string {
  if (all.length === 0) {
    return `<p class="note">No cached usage found. Run Claude Code once to populate it.</p>`;
  }
  const limits = chosen(all, s)!;
  const meters = limits.meters.filter((m) => wanted(m, s));

  let previousReset = "";
  let scopedSeen = 0;
  const bars = meters
    .map((m) => {
      const hue = hueFor(m, scopedSeen);
      if (m.kind !== "session" && m.kind !== "weekly_all") scopedSeen += 1;
      const html = meter(m, previousReset, hue);
      previousReset = resetLabel(m.resetsAt) || previousReset;
      return html;
    })
    .join("");

  return `
    ${accountPicker(all, limits)}
    ${bars || `<p class="note">Nothing selected. Use the sliders to turn a meter back on.</p>`}
    ${s.provenance ? provenance(limits) : ""}`;
}

/** The boolean switches, as distinct from the account and interval settings. */
type SectionKey = "session" | "weekly" | "provenance";

/** Every switchable item, in the order the panel lists them. */
const SECTIONS: ReadonlyArray<[SectionKey, string]> = [
  ["session", "Current session"],
  ["weekly", "Weekly limits"],
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
    </section>`;
}

const bodyEl = document.getElementById("body")!;
const contentEl = document.getElementById("content")!;
const clockEl = document.getElementById("clock")!;

/** Applies the clock's position, or takes it off the card entirely. */
function applyClockPosition(): void {
  clockEl.hidden = settings.clock === "off";
  clockEl.dataset.align = settings.clock;
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
    clockEl.offsetHeight +
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
  account: null,
  refreshSeconds: 120,
  clock: "right",
};
let showingSettings = false;
let accounts: Limits[] = [];

async function draw(): Promise<void> {
  contentEl.innerHTML = showingSettings ? renderSettings(settings) : render(accounts, settings);
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
    accounts = await invoke<Limits[]>("plan_limits");
  } catch (err) {
    contentEl.innerHTML = `<p class="note">Could not read usage: ${esc(String(err))}</p>`;
    await fitWindow();
    return;
  }
  await draw();
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
  if (target instanceof HTMLSelectElement && target.id === "clock-position") {
    settings.clock = target.value;
    persist();
    applyClockPosition();
    // Showing or hiding the clock changes how tall the card is.
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
document.getElementById("close")!.addEventListener("click", () => void getCurrentWindow().close());

tick();
setInterval(tick, 1000);

void (async () => {
  try {
    settings = await invoke<Settings>("load_settings");
  } catch {
    // Keep the defaults; everything stays visible.
  }
  applyClockPosition();
  scheduleRefresh();
  await load();
})();
