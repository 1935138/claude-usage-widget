import { describe, expect, it } from "vitest";

import { accountName, chosen, identity } from "./accounts";
import type { Limits, Settings } from "./types";

const limits = (over: Partial<Limits> = {}): Limits => ({
  meters: [],
  fetchedAt: new Date().toISOString(),
  source: "Windows",
  account: "c8abb3bc",
  email: "you@example.com",
  organization: null,
  plan: null,
  live: true,
  extraUsage: null,
  reason: null,
  ...over,
});

const settings = (account: string | null): Settings => ({
  session: true,
  weekly: true,
  provenance: true,
  pace: true,
  account,
  refreshSeconds: 300,
  clock: "right",
  timeFormat: "24",
  cornerRadius: 12,
});

describe("identity", () => {
  it("is the email, which is what the backend keys on", () => {
    expect(identity(limits())).toBe("you@example.com");
  });

  it("falls back to the account id where a cache recorded no email", () => {
    expect(identity(limits({ email: null }))).toBe("c8abb3bc");
  });
});

describe("chosen", () => {
  const windows = limits({ email: "you@example.com", account: "c8abb3bc" });
  const other = limits({ email: "someone@example.com", account: "9fb0e902" });

  it("follows the freshest reading when nothing is chosen", () => {
    expect(chosen([windows, other], settings(null))).toBe(windows);
  });

  it("picks the account out by email", () => {
    expect(chosen([windows, other], settings("someone@example.com"))).toBe(other);
  });

  it("still honours an account id stored by an older version", () => {
    // Settings files written before the switch to email name a short uuid.
    expect(chosen([windows, other], settings("9fb0e902"))).toBe(other);
  });

  it("falls back to the freshest when the stored account is gone", () => {
    expect(chosen([windows], settings("someone@example.com"))).toBe(windows);
  });

  it("has nothing to choose from an empty list", () => {
    expect(chosen([], settings(null))).toBeUndefined();
  });
});

describe("accountName", () => {
  it("prefers the email, which tells two people apart", () => {
    expect(accountName(limits())).toBe("you@example.com");
    expect(accountName(limits({ email: null }))).toBe("c8abb3bc");
  });
});
