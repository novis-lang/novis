// The note a tool leaves when it writes a tracked file behind a shell call that names no path. A loop
// driver names its ledger on a session's environment: this repository's driver through `NOVIS_LOOP_WRITES`,
// looper through `LOOPER_WRITTEN`. Each sweep reads its own ledger to tell the session's own writes from a
// person's, so every path goes to each ledger that is named.

import { appendFileSync } from "node:fs";
import { resolve } from "node:path";
import { rel } from "./paths.ts";

export const ENV = "NOVIS_LOOP_WRITES";
export const LOOPER_ENV = "LOOPER_WRITTEN";

/**
 * Notes that this process wrote `paths`, in the spelling `git status` uses, in the ledger each of `ENV`
 * and `LOOPER_ENV` names. Never throws and never reports: the caller's real work is already done, and a
 * bookkeeping note that failed must not turn a successful write into a non-zero exit.
 */
export function record(...paths: string[]): void {
  if (paths.length === 0) return;
  const ledgers = [process.env[ENV], process.env[LOOPER_ENV]].filter((l): l is string => !!l);
  if (ledgers.length === 0) return;
  const text = paths.map((p) => `${rel(resolve(p))}\n`).join("");
  for (const ledger of new Set(ledgers)) {
    try {
      appendFileSync(ledger, text, "utf8");
    } catch {
      // The note is best-effort, as above.
    }
  }
}
