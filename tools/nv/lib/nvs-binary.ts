// The `nvs` binary a check reads when none is named: the newest built of the debug, release, proof and
// covws debug builds. `target/release/` is rebuilt only after a goal is retired, and each stage's
// `cargo build` writes `target/debug/`, so a fixed order that put release first read the previous
// goal's binary while the goal's own gate ran, and a feature the goal added owed no proof locally.

import { existsSync, statSync } from "node:fs";
import { join } from "node:path";
import { COVWS_TARGET, hostTriple } from "./covws.ts";
import { ROOT } from "./paths.ts";

const EXE = process.platform === "win32" ? "nvs.exe" : "nvs";

/** Each build an `nvs` can come from, repo-relative. */
export const BUILT_NVS = [`target/debug/${EXE}`, `target/release/${EXE}`, `target/proof/${EXE}`, `${COVWS_TARGET}/${hostTriple()}/debug/${EXE}`];

/** The absolute path of the most recently written of `candidates` under `root`, or null when none
 * exists. A tie goes to the earlier candidate. */
export function newestNvs(root: string = ROOT, candidates: string[] = BUILT_NVS): string | null {
  let best: { path: string; mtime: number } | null = null;
  for (const c of candidates) {
    const path = join(root, c);
    if (!existsSync(path)) continue;
    const mtime = statSync(path).mtimeMs;
    if (best === null || mtime > best.mtime) best = { path, mtime };
  }
  return best?.path ?? null;
}
