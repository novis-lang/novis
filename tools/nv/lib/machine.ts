// How wide this machine may run something, probed once and remembered.
//
// ## Why this exists
//
// A parallel width that is right on one box does not travel: four workers is most of an 8-core laptop
// and a third of what a 32-core one could give. So no caller holds a constant, and this file holds the
// policy instead. What a machine *is* -- how many cores the work will actually see, how much memory is
// free where it runs, what one unit of that work costs serially -- is measured once and cached in
// `.loop/machine.json`, because none of it changes between runs and probing it is the only part with a
// cost.
//
// ## The policy, and its one home
//
// `width` is the only place a parallel width is decided, for every caller. **Half the cores the work
// will actually see**, never fewer than two, never more than there are items to run, and never more
// than free memory divided by what one worker was measured to hold.
//
// Half, and not more, because the machine is not idle. The loop starts a background release build
// before the valgrind sweep and lets the two overlap deliberately -- that build is the longest pole of
// an acceptance check, and cores handed to the sweep come straight out of it. The sweep's own scaling
// flattens well before the box is full, so three quarters would buy little of a check the build already
// dominates. `docs/agent/coordinator.md` holds the measurement.
//
// The floor of two is for small boxes: a 4-core runner still halves its sweep, and only a genuinely
// single-core machine runs anything serially.
//
// ## What is cached, and when it is not
//
// One entry per *execution context* -- `wsl` and `native` are different machines as far as this is
// concerned, and on Windows they are different core counts (WSL2 takes its CPU and memory limits from
// `.wslconfig`, not from the host). An entry is re-probed when the host name changes, when the format
// version moves, or after `STALE_DAYS`, which is the cheap way to notice a `.wslconfig` edit. A caller
// may fold its own measurements into an entry with `remember`, and every key it wrote survives a save.
//
// `NVS_JOBS` overrides every width; a caller's own variable (`NVS_VALGRIND_JOBS`, `NVS_TRY_JOBS`)
// overrides its alone. Each is for a one-off -- none is written back to the cache.

import { spawnSync } from "node:child_process";
import { cpus, hostname } from "node:os";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { ROOT } from "./paths.ts";

/** Under `.loop/`, beside the sweep's memo: git-ignored, per clone, and never pruned. */
export const CACHE_FILE = join(ROOT, ".loop", "machine.json");

/** The cache format. Bumped when a probe learns a new field the width depends on. */
export const VERSION = 1;

/** The policy, in three numbers. The module doc says why it is half and not more. */
export const FRACTION = 0.5;
export const FLOOR = 2;
export const STALE_DAYS = 30;

/** What a probe reports, and the only keys read back out of what it printed. */
const NUMERIC = ["cores", "mem_kb", "sample_s", "sample_rss_kb", "sample_code"];

/** One context's facts. Keys a caller folded in with `remember` ride along untouched. */
export type Entry = Record<string, unknown> & {
  cores?: number;
  mem_kb?: number;
  sample_s?: number;
  sample_rss_kb?: number;
  sample_pending?: number;
  host?: string;
  probed?: string;
};

export interface Doc {
  version?: number;
  contexts?: Record<string, Entry>;
  [key: string]: unknown;
}

// ------------------------------------------------------------------------------------ the policy

export interface WidthOptions {
  ceiling?: number | null | undefined;
  memKb?: number | null | undefined;
  workerKb?: number | null | undefined;
  fraction?: number;
  floor?: number;
}

/**
 * How many of `cores` this may use, under every cap that applies. `ceiling` is how many items there
 * are -- more workers than work is only process launches. `memKb` with `workerKb` caps on memory: half
 * of what was free when the box was probed, divided by what one worker was measured to hold. Either
 * missing means no memory cap, which is the right default rather than a guessed one: at half the cores
 * it has never been the binding constraint on a machine that could run the work at all.
 */
export function width(cores: number | null | undefined, opts: WidthOptions = {}): number {
  const { ceiling, memKb, workerKb, fraction = FRACTION, floor = FLOOR } = opts;
  const n = Math.max(1, Math.trunc(cores || 1));
  let want = Math.max(n > 1 ? floor : 1, Math.trunc(n * fraction));
  if (ceiling) want = Math.min(want, Math.max(1, Math.trunc(ceiling)));
  if (memKb && workerKb) want = Math.min(want, Math.max(1, Math.floor(Math.floor(memKb) / 2 / Math.max(1, Math.trunc(workerKb)))));
  return Math.max(1, Math.min(want, n));
}

/** The first of `names` set in the environment to a positive integer, or null. */
export function override(...names: string[]): number | null {
  for (const name of names) {
    const raw = (process.env[name] ?? "").trim();
    if (/^\d+$/.test(raw) && Number(raw) > 0) return Number(raw);
  }
  return null;
}

// --------------------------------------------------------------------------------------- probing

/**
 * One shell line, run where the work runs. No double quotes and no newlines anywhere in it: on Windows
 * this reaches a Linux shell as one argv element of `wsl.exe`, and the fewer layers that can reinterpret
 * it the better. Every part degrades to silence rather than to an error, so a probe on a box without
 * `/proc` or without `/usr/bin/time` simply reports less.
 */
export const PROBE_SH =
  "echo cores $(nproc 2>/dev/null || echo 0); " + "grep MemAvailable /proc/meminfo 2>/dev/null | sed s/MemAvailable:/mem_kb/";

/**
 * The serial baseline, appended to `PROBE_SH` when the caller hands over one unit of the real work. It
 * answers two questions in the one call the probe already costs: what a single item costs with nothing
 * else running -- which is what makes a later sweep's speedup a measurement rather than an estimate --
 * and what one worker holds, which is where the memory cap comes from.
 */
const SAMPLE_SH = (sample: string) =>
  "; if command -v /usr/bin/time >/dev/null 2>&1; then " +
  "/usr/bin/time -o /tmp/nvs-machine-probe -f 'sample_s %e sample_rss_kb %M' " +
  `sh -c '${sample}' >/dev/null 2>&1; echo sample_code $?; ` +
  "cat /tmp/nvs-machine-probe 2>/dev/null; rm -f /tmp/nvs-machine-probe; fi";

/** The whole probe as one shell line -- facts, and a timed sample when there is one. A sample holding a
 * single quote cannot ride inside `sh -c '...'`, so it is left out and only the facts are probed. */
export function probeSh(sample?: string | null): string {
  if (!sample || sample.includes("'")) return PROBE_SH;
  return PROBE_SH + SAMPLE_SH(sample);
}

/**
 * `{field: number}` out of whatever the probe managed to print. Deliberately forgiving: every field is
 * optional, `mem_kb 15000 kB` and `cores 16` and `sample_s 3.21 sample_rss_kb 55120` all read the same
 * way, and anything unrecognised is ignored rather than being an error. A probe that prints nothing is a
 * machine with one core as far as `width` is concerned, which is the safe direction to be wrong in.
 */
export function readProbe(text: string | null | undefined): Record<string, number> {
  const found: Record<string, number> = {};
  for (const m of (text ?? "").matchAll(/\b([a-z_]+)\s+(\d+(?:\.\d+)?)/g)) {
    const [, key, value] = m as unknown as [string, string, string];
    if (NUMERIC.includes(key) && !(key in found)) found[key] = Number(value);
  }
  return found;
}

/** How a caller reaches the context being probed: `wsl.exe --exec bash -lc` for the WSL leg, a local
 * `bash -lc` for a native Linux one. */
export type Runner = (line: string) => { code: number; text: string };

/** Run the probe through `runner` and read what came back. */
export function posixProbe(runner: Runner, sample?: string | null): Entry {
  const { code, text } = runner(probeSh(sample));
  const got: Entry = readProbe(text);
  if (code !== 0 && Object.keys(got).length === 0) return {};
  const sampleCode = got.sample_code ?? 0;
  delete got.sample_code;
  if (sampleCode !== 0) {
    // The sample ran and failed. Its timing says nothing about the cost of the work, and the failure
    // itself is the caller's business -- the sweep will hit the same fixture. The entry is marked so the
    // next run measures again: a red tree is temporary.
    delete got.sample_s;
    delete got.sample_rss_kb;
    got.sample_pending = 1;
  }
  // No `sample_code` at all means the box has no `/usr/bin/time`, which will still be true tomorrow.
  // That entry is complete as it stands: the width never needed a baseline, only the memory cap did.
  return got;
}

/** `bash -lc <line>` on this machine, as a `Runner`. */
export const bash: Runner = (line) => {
  const p = spawnSync("bash", ["-lc", line], { encoding: "utf8", timeout: 300_000 });
  if (p.error) return { code: 1, text: "" };
  return { code: p.status ?? 1, text: (p.stdout ?? "") + (p.stderr ?? "") };
};

/**
 * This process's own machine, without leaving the process. No sample: the only caller that runs work
 * here is `bun nv try`, whose unit of work is a snippet that takes milliseconds and holds nothing, and a
 * baseline is worth a probe's cost for valgrind and not for that.
 */
export function localProbe(): Entry {
  const got: Entry = { cores: cpus().length || 1 };
  try {
    const line = readFileSync("/proc/meminfo", "utf8")
      .split("\n")
      .find((l) => l.startsWith("MemAvailable:"));
    const kb = Number(line?.split(/\s+/)[1]);
    if (line && Number.isInteger(kb)) got.mem_kb = kb;
  } catch {
    // No `/proc`: this is Windows or a BSD, and the width has no memory cap here.
  }
  return got;
}

// ----------------------------------------------------------------------------------------- cache

/** Today as `YYYY-MM-DD`, in local time. */
export function today(now: Date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/** The cache as it stands, or an empty one when it is missing, unreadable or of another version. */
export function load(cache: string = CACHE_FILE): Doc {
  try {
    const doc = JSON.parse(readFileSync(cache, "utf8"));
    return doc && typeof doc === "object" && !Array.isArray(doc) && doc.version === VERSION ? doc : {};
  } catch {
    return {};
  }
}

function sorted(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sorted);
  if (value && typeof value === "object") {
    const o = value as Record<string, unknown>;
    return Object.fromEntries(Object.keys(o).sort().map((k) => [k, sorted(o[k])]));
  }
  return value;
}

/** The cache's text: sorted keys, one space of indent, which is the shape every reader of it expects. */
export function dump(doc: Doc): string {
  return JSON.stringify(sorted(doc), null, 1);
}

/** Write `doc` back as the current format version. */
export function save(doc: Doc, cache: string = CACHE_FILE): void {
  doc.version = VERSION;
  try {
    mkdirSync(dirname(cache), { recursive: true });
    writeFileSync(cache, dump(doc));
  } catch {
    // A machine profile is an optimisation; failing to keep it is not a failure.
  }
}

/**
 * Whether `entry` must be probed again. A different host is the obvious case; the age is for the one
 * that is not observable -- a `.wslconfig` edit changes how many cores a leg sees and leaves no trace
 * this side of the probe.
 */
export function stale(entry: Entry | undefined, now: Date = new Date(), days: number = STALE_DAYS): boolean {
  if (!entry || typeof entry !== "object" || !entry.cores) return true;
  if (entry.host !== hostname()) return true;
  if (entry.sample_pending) return true; // the baseline could not be taken last time; try once more
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(entry.probed ?? "");
  if (!m) return true;
  const probed = Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  const [y, mo, d] = today(now).split("-").map(Number) as [number, number, number];
  return (Date.UTC(y, mo - 1, d) - probed) / 86_400_000 > days;
}

export interface ProfileOptions {
  probe?: (() => Entry) | null;
  refresh?: boolean;
  cache?: string;
}

/**
 * The cached facts for one execution context, probing when there is nothing usable. `context` is where
 * the work runs -- `wsl`, `native`, `local` -- and not which caller is asking: two tools running work in
 * the same place are looking at the same machine.
 */
export function profile(context: string, opts: ProfileOptions = {}): Entry {
  const { probe = null, refresh = false, cache = CACHE_FILE } = opts;
  const doc = load(cache);
  const entries = (doc.contexts ??= {});
  const entry = entries[context];
  const usable = entry && typeof entry === "object" && entry.cores ? entry : null;
  if (!refresh && !stale(entry)) return entry!;
  // Nobody can reach a context other than `local` from here without its probe -- only the tool that
  // runs work there can probe it. Whatever it left behind is better than a guess.
  if (probe === null && context !== "local") return usable ?? { cores: 1 };
  const got = (probe ?? localProbe)();
  // Nothing usable came back. Whatever was cached stays: a WSL that was busy for one probe is still the
  // same machine it was.
  if (!got.cores) return usable ?? { cores: 1 };
  const fresh: Entry = { host: hostname(), probed: today(), ...got };
  entries[context] = fresh;
  save(doc, cache);
  return fresh;
}

/** Fold measured numbers into a context's entry -- a sweep's real speedup, say -- without re-probing
 * anything. Does nothing when the context has never been probed. */
export function remember(context: string, fields: Record<string, unknown>, cache: string = CACHE_FILE): void {
  const doc = load(cache);
  const entry = doc.contexts?.[context];
  if (!entry || typeof entry !== "object") return;
  for (const [k, v] of Object.entries(fields)) if (v !== null && v !== undefined) entry[k] = v;
  save(doc, cache);
}

export interface JobsOptions extends ProfileOptions {
  ceiling?: number | null;
  /** The caller's own override variables, read before `NVS_JOBS`. */
  envs?: string[];
}

/** The width for work in `context`: an override if one is set, the policy otherwise. */
export function jobs(context: string, opts: JobsOptions = {}): number {
  const { ceiling = null, envs = [] } = opts;
  const forced = override(...envs, "NVS_JOBS");
  if (forced) return ceiling ? Math.max(1, Math.min(forced, ceiling)) : forced;
  const p = profile(context, opts);
  return width(p.cores, { ceiling, memKb: p.mem_kb, workerKb: p.sample_rss_kb });
}
