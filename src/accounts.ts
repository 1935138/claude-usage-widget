// Which account's figures the card is showing.

import type { Limits, Settings } from "./types";

/**
 * What makes two readings the same account, matching the backend's own rule.
 *
 * The short account id is not it: one login can be recorded under a different
 * `account_uuid` by each install, so keying on it loses track of an account the
 * moment a different install's reading wins.
 */
export const identity = (l: Limits): string => l.email ?? l.account;

/** The account whose meters are shown: the chosen one, else the freshest. */
export function chosen(all: Limits[], s: Settings): Limits | undefined {
  // `account` held a short account id before it held an email; a stored one of
  // either kind still picks its account out.
  return (
    all.find((l) => identity(l) === s.account) ?? all.find((l) => l.account === s.account) ?? all[0]
  );
}

/** The email, or the account id for a cache that recorded none. */
export const accountName = (l: Limits): string => l.email ?? l.account;
