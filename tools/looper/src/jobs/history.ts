// The history of each item of a project's job progress, and the estimate of the time a job has left,
// which reads it. It ports `crates/looper-store/src/items.rs` from looper's database to the project's
// `.looper/history/`.
//
// - An item is keyed by its id, which stays the same across runs. Its runs are one state file,
//   `<sha-1 of the id>.json`, holding `{"id", "runs"}`, and each run is `{"green", "took_ms", "at"}`.
// - A job's runner records each item of its own fold that ended green or red, once the job ends
//   (`runsOf`, `record`). A skipped, cancelled or kept item ran no whole time, so it is not recorded.
// - `medians` gives each asked item's median time over the runs it keeps.
// - `estimate` sums the medians of the steps left, the items nothing nests above. A step's time spans
//   its units, the build before them and the units that run at once, which a sum of its units misses.
//   A running step adds what its median has left after the time it ran so far, and none once it ran
//   past it. A step with no history weighs the mean of the run's other steps' medians.
//
// # The bounds it keeps
//
// - An item keeps its newest `RUNS_KEPT` runs.
// - `prune` deletes every item whose newest run ended more than `UNSEEN_MS` ago. `record` runs it at
//   most once per `PRUNE_EVERY_MS`, which the file `pruned` records.
// - `medians` holds one item's runs at a time, and one median per item asked for.
//
// # Known gaps
//
// - Two runners that record one item at the same moment race on its file, and one run may be lost.

import { createHash } from "node:crypto";
import { readdirSync, unlinkSync } from "node:fs";
import { join } from "node:path";
import { readState, writeState } from "../store/state.ts";
import type { Fold } from "./fold.ts";

/** The most runs one item keeps. */
export const RUNS_KEPT = 20;
/** How long an item stays with no new run before `prune` deletes it: thirty days. */
export const UNSEEN_MS = 30 * 24 * 3600 * 1000;
/** How long a prune holds before `record` runs the next: 10 minutes. */
export const PRUNE_EVERY_MS = 10 * 60 * 1000;
/** The time of the last prune, in the history's folder. */
export const PRUNED = "pruned";

/** One run of an item that ended green or red. */
export interface Ran {
  item: string;
  green: boolean;
  took_ms: number;
  /** When it ended, in unix ms. */
  at: number;
}

/** One item's file. */
interface Runs {
  id: string;
  runs: { green: boolean; took_ms: number; at: number }[];
}

/** The history folder of the project at `root`. */
export function historyDir(root: string): string {
  return join(root, ".looper", "history");
}

/** The file of item `id` in the history folder `dir`. */
function fileOf(dir: string, id: string): string {
  return join(dir, `${createHash("sha1").update(id).digest("hex")}.json`);
}

/** The runs of item `id` in `dir`, none when its file is missing or does not parse. */
function runsIn(dir: string, id: string): Runs["runs"] {
  try {
    const doc = readState<Runs>(fileOf(dir, id));
    return doc?.id === id && Array.isArray(doc.runs) ? doc.runs : [];
  } catch {
    return [];
  }
}

/** The runs worth recording from `fold`, a job's finished fold: each item that ended green or red. */
export function runsOf(fold: Fold): Ran[] {
  const out: Ran[] = [];
  for (const item of fold.items()) {
    if (item.state !== "ended" || (item.result !== "green" && item.result !== "red")) continue;
    if (item.started === null || item.ended === null) continue;
    out.push({ item: item.id, green: item.result === "green", took_ms: Math.max(0, item.ended - item.started), at: item.ended });
  }
  return out;
}

/** Adds `runs` to the history of the project at `root`. Each item then drops its runs past the newest `RUNS_KEPT`. */
export function record(root: string, runs: readonly Ran[], now = Date.now()): void {
  const dir = historyDir(root);
  for (const run of runs) {
    const kept = [...runsIn(dir, run.item), { green: run.green, took_ms: run.took_ms, at: run.at }].slice(-RUNS_KEPT);
    writeState(fileOf(dir, run.item), { id: run.item, runs: kept } satisfies Runs);
  }
  if (runs.length > 0) pruneDue(root, now);
}

/** Deletes every item of the project at `root` whose newest run ended more than `UNSEEN_MS` before `now`. Gives how many. */
export function prune(root: string, now = Date.now()): number {
  const dir = historyDir(root);
  let names: string[];
  try {
    names = readdirSync(dir).filter((n) => /^[0-9a-f]{40}\.json$/.test(n));
  } catch {
    return 0;
  }
  let dropped = 0;
  for (const name of names) {
    let doc: Runs | undefined;
    try {
      doc = readState<Runs>(join(dir, name));
    } catch {
      doc = undefined;
    }
    const newest = Math.max(0, ...(doc?.runs ?? []).map((r) => r.at));
    if (newest >= now - UNSEEN_MS) continue;
    try {
      unlinkSync(join(dir, name));
      dropped++;
    } catch {}
  }
  return dropped;
}

/** Prunes when the last prune is `PRUNE_EVERY_MS` old, or there was none. */
function pruneDue(root: string, now: number): void {
  const path = join(historyDir(root), PRUNED);
  let last: unknown;
  try {
    last = readState<number>(path);
  } catch {
    last = undefined;
  }
  if (typeof last === "number" && now - last < PRUNE_EVERY_MS && last <= now) return;
  writeState(path, now);
  prune(root, now);
}

/** The median time of each of `items` in the history of the project at `root`. An item with no runs is left out. */
export function medians(root: string, items: Iterable<string>): Map<string, number> {
  const dir = historyDir(root);
  const out = new Map<string, number>();
  for (const item of items) {
    const m = median(runsIn(dir, item).map((r) => r.took_ms));
    if (m !== null) out.set(item, m);
  }
  return out;
}

/** The median of `values`: the mean of the middle two, rounded down, when their count is even. */
export function median(values: readonly number[]): number | null {
  if (values.length === 0) return null;
  const sorted = values.toSorted((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? Math.floor(((sorted[mid - 1] as number) + (sorted[mid] as number)) / 2) : (sorted[mid] as number);
}

/** One step a job has left. */
export interface Left {
  item: string;
  /** How long it has run so far, while it runs. */
  running_ms: number | null;
}

/** The time a job has left. */
export interface Estimate {
  left_ms: number;
  /** How many of the steps left have a history of their own. */
  rests_on: number;
  /** How many steps are left. */
  items: number;
}

/**
 * The time the steps `left` take, from `known`, each item's median that has a history. `steps` is every
 * step of the run, ended or not, and a step left with no history weighs the mean of their medians.
 * Null when no step of the run has one.
 */
export function estimate(known: ReadonlyMap<string, number>, steps: readonly string[], left: readonly Left[]): Estimate | null {
  const have = steps.map((s) => known.get(s)).filter((m): m is number => m !== undefined);
  if (have.length === 0) return null;
  const fallback = Math.floor(have.reduce((a, b) => a + b, 0) / have.length);
  const est: Estimate = { left_ms: 0, rests_on: 0, items: left.length };
  for (const l of left) {
    const own = known.get(l.item);
    if (own !== undefined) est.rests_on++;
    est.left_ms += Math.max(0, (own ?? fallback) - (l.running_ms ?? 0));
  }
  return est;
}
