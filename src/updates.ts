// Checking whether a newer release is out, and taking it.
//
// Quiet by default: the check happens when the settings panel is opened and the
// answer sits there as a line of text. A widget that interrupts to talk about
// itself is worse than one that is a version behind.

import { check, type Update } from "@tauri-apps/plugin-updater";

export type UpdateStatus =
  | { kind: "unknown" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "available"; version: string }
  | { kind: "installing" }
  | { kind: "failed" };

/** The update the last check turned up, kept for the install that may follow. */
let pending: Update | null = null;

export async function checkForUpdate(): Promise<UpdateStatus> {
  try {
    pending = await check();
    return pending ? { kind: "available", version: pending.version } : { kind: "current" };
  } catch {
    // A failed check is not worth a message of its own: no network, a release
    // without the metadata, a signature that does not verify.
    pending = null;
    return { kind: "failed" };
  }
}

/**
 * Downloads and installs what the last check found.
 *
 * On Windows this hands over to the installer, which closes the widget and
 * starts the new one, so this call does not usually return.
 */
export async function installUpdate(): Promise<UpdateStatus> {
  if (!pending) return { kind: "unknown" };
  try {
    await pending.downloadAndInstall();
    return { kind: "installing" };
  } catch {
    return { kind: "failed" };
  }
}
