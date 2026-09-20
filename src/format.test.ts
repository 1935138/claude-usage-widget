import { describe, expect, it } from "vitest";

import {
  clockTime,
  elapsedShare,
  esc,
  extraUsagePhrase,
  provenance,
  resetLabel,
  STALE_MS,
} from "./format";
import { strings } from "./i18n";
import type { Limits, Meter } from "./types";

const t = strings("en");

const meter = (over: Partial<Meter> = {}): Meter => ({
  kind: "session",
  label: "Current session",
  percent: 50,
  severity: "normal",
  resetsAt: null,
  windowSeconds: null,
  isActive: false,
  scopeModel: null,
  ...over,
});

const limits = (over: Partial<Limits> = {}): Limits => ({
  meters: [],
  fetchedAt: new Date().toISOString(),
  source: "Windows",
  configDir: "",
  account: "c8abb3bc",
  email: "you@example.com",
  organization: null,
  plan: "Max 5x",
  live: true,
  extraUsage: null,
  reason: null,
  ...over,
});

describe("esc", () => {
  it("escapes what would otherwise close a tag or an attribute", () => {
    expect(esc('<b class="x">&</b>')).toBe("&lt;b class=&quot;x&quot;&gt;&amp;&lt;/b&gt;");
  });

  it("leaves ordinary text alone", () => {
    expect(esc("Current week (Fable)")).toBe("Current week (Fable)");
  });
});

describe("clockTime", () => {
  // The locale decides the wording; the setting decides the clock. Asserting on
  // shape keeps this true wherever the tests run.
  const at = new Date("2026-09-17T17:25:30");

  it("writes 24-hour times without a meridiem", () => {
    expect(clockTime(at, "24", t)).toMatch(/^\d{2}:\d{2}$/);
  });

  it("writes 12-hour times with one", () => {
    expect(clockTime(at, "12", t)).toMatch(/^\d{1,2}:\d{2}\s?(AM|PM)$/i);
  });

  it("adds seconds only when asked", () => {
    expect(clockTime(at, "24", t, true)).toMatch(/^\d{2}:\d{2}:\d{2}$/);
  });
});

describe("resetLabel", () => {
  it("says nothing when there is no reset instant", () => {
    expect(resetLabel(null, "24", t)).toBe("");
  });

  it("says nothing for an instant it cannot read", () => {
    expect(resetLabel("the day after tomorrow", "24", t)).toBe("");
  });

  it("gives the time alone for a reset later today", () => {
    const later = new Date(Date.now() + 60 * 60_000);
    expect(resetLabel(later.toISOString(), "24", t)).toMatch(/^Resets \d{2}:\d{2}$/);
  });

  it("names the day for a reset beyond today", () => {
    const nextWeek = new Date(Date.now() + 7 * 24 * 60 * 60_000);
    expect(resetLabel(nextWeek.toISOString(), "24", t)).toMatch(/^Resets .+, \d{2}:\d{2}$/);
  });
});

describe("elapsedShare", () => {
  it("is unknown without a window to measure against", () => {
    expect(elapsedShare(meter({ resetsAt: new Date().toISOString() }))).toBeNull();
    expect(elapsedShare(meter({ windowSeconds: 18000 }))).toBeNull();
  });

  it("is the share of the window already spent", () => {
    const resets = new Date(Date.now() + 2.5 * 60 * 60_000).toISOString();
    const share = elapsedShare(meter({ resetsAt: resets, windowSeconds: 5 * 60 * 60 }));
    expect(share).toBeCloseTo(0.5, 2);
  });

  it("is unknown for a reset that has already passed", () => {
    // A marker placed from a stale instant would sit somewhere meaningless.
    const resets = new Date(Date.now() - 60_000).toISOString();
    expect(elapsedShare(meter({ resetsAt: resets, windowSeconds: 18000 }))).toBeNull();
  });
});

describe("extraUsagePhrase", () => {
  it("says nothing when the cache carried no credit state", () => {
    expect(extraUsagePhrase(null, t)).toBe("");
  });

  it("gives the share spent when there is one", () => {
    expect(extraUsagePhrase({ enabled: true, disabledReason: null, percent: 12.4 }, t)).toBe(
      "extra usage 12%",
    );
  });

  it("settles for on when the share is unknown", () => {
    expect(extraUsagePhrase({ enabled: true, disabledReason: null, percent: null }, t)).toBe(
      "extra usage on",
    );
  });

  it("reads the reason back as words", () => {
    expect(
      extraUsagePhrase({ enabled: false, disabledReason: "out_of_credits", percent: null }, t),
    ).toBe("extra usage off (out of credits)");
  });
});

describe("provenance", () => {
  it("calls a live reading updated, and does not call it stale", () => {
    const { text, stale } = provenance(limits(), "24", t);
    expect(text).toMatch(/^Updated · \d{2}:\d{2}$/);
    expect(stale).toBe(false);
  });

  it("dates a cache old enough to mislead, and says what to do", () => {
    const old = new Date(Date.now() - STALE_MS - 60_000).toISOString();
    const { text, stale } = provenance(limits({ live: false, fetchedAt: old }), "24", t);
    expect(stale).toBe(true);
    expect(text).toMatch(/^Cached · .+ \d{2}:\d{2} · run \/usage to refresh$/);
  });

  it("names the reason instead when the backend gave one", () => {
    const old = new Date(Date.now() - STALE_MS - 60_000).toISOString();
    const { text } = provenance(limits({ live: false, fetchedAt: old, reason: "expired" }), "24", t);
    expect(text).toContain("sign-in expired");
    expect(text).not.toContain("run /usage");
  });

  it("puts the source and account in the tooltip, not the line", () => {
    const { text, detail } = provenance(limits({ source: "WSL: Ubuntu" }), "24", t);
    expect(detail).toBe("WSL: Ubuntu · account c8abb3bc");
    expect(text).not.toContain("WSL");
  });
});
