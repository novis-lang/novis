// The tools' own gate as atoms: `tsc` over every module under `tools/nv` (`step:nv`), and each `bun test`
// file of the tools (`nvtest:<file>`) run in a process of its own. `nv verify`'s `nv` step and the
// sweep's `bun nv selftest` and `bun test <file>` checks run them through here, so a run of either
// answers both.
//
// A test file runs with `lib/reads-preload.ts`, so what it read of the tree, the modules it loaded and
// what any `bun nv` process it started read go into its reads log, and any `nvs` it ran into its
// coverage. So a test file that starts `git` holds the whole tree alone, and the rest are selected by
// what each one read.

import { rmSync } from "node:fs";
import { join } from "node:path";
import { digest } from "../keys/scan.ts";
import { covwsNvs } from "../lib/covws.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { recordName } from "../proofs/run.ts";
import { fileDef, nvTestId } from "./atoms.ts";
import type { Recorder } from "./record.ts";
import { nvKeys } from "./seed.ts";
import type { Keyed, Verdict } from "./store.ts";

/** `tsc` over the tools: the `nv` step's own atom. */
export const NV_TSC = "step:nv";
const TSC = join(ROOT, "node_modules", "typescript", "bin", "tsc");
/** What `tsc` reads besides the modules under `tools/nv`. */
const TSC_INPUTS = ["package.json", "bun.lock", "tsconfig.json", "bunfig.toml"];
const BUN_TEST_RE = /^\s*(\d+) pass\s*$[\s\S]*?^\s*(\d+) fail\s*$/m;
const TIMEOUT_MS = 10 * 60 * 1000;

export interface ToolRun {
  code: number;
  out: string;
  verdict: Verdict;
}

/** Runs `tsc --noEmit` over the tools and records `step:nv`. */
export async function runTsc(rec: Recorder): Promise<ToolRun> {
  const p = await run([process.execPath, TSC, "--noEmit", "-p", "tsconfig.json"], { timeoutMs: TIMEOUT_MS, reap: true });
  const keys: Keyed = new Map([["tree:tools/nv", ""]]);
  for (const f of TSC_INPUTS) {
    keys.set(`file:${f}`, "");
    keys.set(`exists:${f}`, "");
  }
  const verdict: Verdict = p.code === 0 && !p.timedOut ? "green" : "red";
  rec.record(NV_TSC, digest("tsc --noEmit -p tsconfig.json"), verdict, keys);
  return { code: p.timedOut ? -1 : p.code, out: p.stdout + p.stderr, verdict };
}

/** Runs one tools test file in a process of its own and records `nvtest:<file>`. */
export async function runNvTest(rec: Recorder, file: string): Promise<ToolRun & { passed: number; failed: number }> {
  const id = nvTestId(file);
  const name = recordName(id);
  const log = join(rec.dir, `${name}.reads`);
  rmSync(log, { force: true });
  const p = await run([process.execPath, "test", "--preload", "./tools/nv/lib/reads-preload.ts", file], {
    env: { ...rec.env(name), NV_READS_LOG: log, NO_COLOR: "1" },
    timeoutMs: TIMEOUT_MS,
    reap: true,
  });
  const keys = nvKeys(log);
  rmSync(log, { force: true });
  for (const [k, d] of (await rec.keysOf(name, [covwsNvs()]))?.keys ?? []) keys.set(k, d);
  keys.set(`file:${file}`, "");
  const out = p.stdout + p.stderr;
  const m = BUN_TEST_RE.exec(out);
  const verdict: Verdict = p.code === 0 && !p.timedOut && m !== null ? "green" : "red";
  rec.record(id, fileDef(file), verdict, keys);
  return { code: p.timedOut ? -1 : p.code, out, verdict, passed: Number(m?.[1] ?? 0), failed: Number(m?.[2] ?? 0) };
}
