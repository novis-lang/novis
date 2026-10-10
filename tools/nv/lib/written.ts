// The note a tool leaves when it writes a tracked file behind a shell call that names no path. Looper names
// its ledger on a session's environment through `LOOPER_WRITTEN`, and its sweep reads the ledger to tell
// the session's own writes from a person's.

import { appendFileSync } from "node:fs";
import { resolve } from "node:path";
import { rel } from "./paths.ts";

export const ENV = "LOOPER_WRITTEN";

/**
 * Notes that this process wrote `paths`, in the spelling `git status` uses, in the ledger `ENV` names.
 * Never throws and never reports: the caller's real work is already done, and a bookkeeping note that
 * failed must not turn a successful write into a non-zero exit.
 */
export function record(...paths: string[]): void {
  const ledger = process.env[ENV];
  if (paths.length === 0 || !ledger) return;
  try {
    appendFileSync(ledger, paths.map((p) => `${rel(resolve(p))}\n`).join(""), "utf8");
  } catch {
    // The note is best-effort, as above.
  }
}
