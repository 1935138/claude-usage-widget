import { describe, expect, it } from "vitest";

import { meterLabel, resolveLang, strings, type Strings } from "./i18n";
import type { Meter } from "./types";

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

describe("resolveLang", () => {
  it("takes the machine's language when nothing was chosen", () => {
    expect(resolveLang("system", "ko-KR")).toBe("ko");
    expect(resolveLang("system", "en-GB")).toBe("en");
  });

  it("settles on English for a language it has no words for", () => {
    expect(resolveLang("system", "fr-FR")).toBe("en");
  });

  it("obeys an explicit choice whatever the machine says", () => {
    expect(resolveLang("en", "ko-KR")).toBe("en");
    expect(resolveLang("ko", "en-US")).toBe("ko");
  });

  it("falls back for a language stored by some later version", () => {
    expect(resolveLang("elvish", "en-US")).toBe("en");
  });
});

describe("meterLabel", () => {
  const t = strings("en");

  it("names the limits `/usage` names", () => {
    expect(meterLabel(meter(), t)).toBe("Current session");
    expect(meterLabel(meter({ kind: "weekly_all" }), t)).toBe("Current week (all models)");
  });

  it("puts the model beside a scoped limit", () => {
    expect(meterLabel(meter({ kind: "weekly_scoped", scopeModel: "Fable" }), t)).toBe(
      "Current week (Fable)",
    );
  });

  it("still names a scoped limit that arrived without a model", () => {
    expect(meterLabel(meter({ kind: "weekly_scoped" }), t)).toBe("Current week (scoped)");
  });

  it("shows a limit kind added after this was written, rather than dropping it", () => {
    expect(meterLabel(meter({ kind: "monthly_thing" }), t)).toBe("monthly thing");
    expect(meterLabel(meter({ kind: "monthly_thing", scopeModel: "Opus" }), t)).toBe(
      "monthly thing (Opus)",
    );
  });

  it("writes them in Korean too", () => {
    const ko = strings("ko");
    expect(meterLabel(meter(), ko)).toBe("현재 세션");
    expect(meterLabel(meter({ kind: "weekly_scoped", scopeModel: "Fable" }), ko)).toBe(
      "이번 주 (Fable)",
    );
  });
});

describe("the catalogues", () => {
  // A missing entry is a type error, but a copy-paste that leaves an English
  // string in the Korean catalogue is not, so check the ones that would show.
  const en = strings("en");
  const ko = strings("ko");

  it("carry the same entries", () => {
    expect(Object.keys(ko).sort()).toEqual(Object.keys(en).sort());
  });

  it("name every reason the backend can report", () => {
    expect(Object.keys(ko.reasons).sort()).toEqual(Object.keys(en.reasons).sort());
  });

  it("are actually translated", () => {
    const sample = (t: Strings) => [t.settings, t.updated, t.cached, t.show, t.language].join("|");
    expect(sample(ko)).not.toBe(sample(en));
  });

  it("count minutes the same way in both", () => {
    expect(en.refreshChoice(0)).toBe("Manual only");
    expect(en.refreshChoice(180)).toBe("3 minutes");
    expect(ko.refreshChoice(180)).toBe("3분");
  });

  it("fall back to pixels for a radius neither names", () => {
    expect(en.cornerChoice(7)).toBe("7px");
    expect(ko.cornerChoice(7)).toBe("7px");
  });
});
