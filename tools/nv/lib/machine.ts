// How wide this machine may run something, probed once and remembered.
//
// ## Why this exists
//
// A parallel width that is right on one box does not travel: four workers is most of an 8-core laptop
// and a third of what a 32-core one could give. So no caller holds a constant, and this file holds the
// policy instead. What a machine *is* -- how many cores the work will actually see, and how much memory
// is free -- is measured once and cached in `.loop/machine.json`, because neither changes between runs.
//
// ## The policy, and its one home
//
// `width` is the only place a parallel width is decided, for every caller. **Half the cores the work
// will actually see**, never fewer than two, and never more than there are items to run.
//
// Half, and not more, because the machine is not idle: a build, a loop's session or another run usually
// shares it. The floor of two is for small boxes: a 4-core runner still runs two at a time, and only a
// genuinely single-core machine runs anything serially.
//
// ## What is cached, and when it is not
//
// One entry per *execution context*, and the one context probed is `local`, this process's own machine.
// An entry is re-probed when the host name changes, when the format version moves, or after
// `STALE_DAYS`.
//
// `NVS_JOBS` overrides every width; a caller's own variable (`NVS_VALGRIND_JOBS`, `NVS_TRY_JOBS`)
// overrides its alone. Each is for a one-off -- none is written back to the cache.

import { cpus, hostname } from "node:os";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { ROOT } from "./paths.ts";

/** Under `.loop/`: git-ignored, per clone, and never pruned. */
export const CACHE_FILE = join(ROOT, ".loop", "machine.json");

/** The cache format. Bumped when the probe learns a new field the width depends on. */
export const VERSION = 1;

/** The policy, in three numbers. The module doc says why it is half and not more. */
export const FRACTION = 0.5;
export const FLOOR = 2;
export const STALE_DAYS = 30;

/** One context's facts. */
export type Entry = Record<string, unknown> & {
  cores?: number;
  mem_kb?: number;
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
  fraction?: number;
  floor?: number;
}

/**
 * How many of `cores` this may use. `ceiling` is how many items there are -- more workers than work is
 * only process launches.
 */
export function width(cores: number | null | undefined, opts: WidthOptions = {}): number {
  const { ceiling, fraction = FRACTION, floor = FLOOR } = opts;
  const n = Math.max(1, Math.trunc(cores || 1));
  let want = Math.max(n > 1 ? floor : 1, Math.trunc(n * fraction));
  if (ceiling) want = Math.min(want, Math.max(1, Math.trunc(ceiling)));
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

/** This process's own machine, without leaving the process: its cores, and its free memory where
 * `/proc` has it. */
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
 * that is not observable, such as a virtual machine given more cores.
 */
export function stale(entry: Entry | undefined, now: Date = new Date(), days: number = STALE_DAYS): boolean {
  if (!entry || typeof entry !== "object" || !entry.cores) return true;
  if (entry.host !== hostname()) return true;
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
 * the work runs, and not which caller is asking: two tools running work in the same place are looking at
 * the same machine.
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
  return width(profile(context, opts).cores, { ceiling });
}
