// A job's progress as one tree, which `exec progress` prints and looper's service reads through its
// `--json`. It is the view of `crates/looper-server/src/progress.rs`, read from the project's store.
//
// - It folds the job's event file afresh (`fold.ts`). A job that ended has its fold finished with its
//   exit, and a lost one as killed, so their open items read as cancelled.
// - Under each item whose `start` names a child job it nests that job's items, and theirs in turn. A
//   job already nested is not nested again, and a child with no record nests nothing.
// - The estimate (`history.ts`) counts only the steps, the items at depth 0. A running step's time so
//   far is taken from `now`.
//
// # The bounds it keeps
//
// - A view holds at most `MAX_ITEMS` items across every job it nests, and nests at most `MAX_DEPTH`
//   jobs deep. It holds one fold at a time beside the items it keeps.

import { join } from "node:path";
import { count, Fold, type Item, MAX_ITEMS, noTotals, type Outcome, type Totals } from "./fold.ts";
import { type Estimate, estimate, type Left, medians } from "./history.ts";
import { EVENTS, type JobState, type Store } from "./store.ts";

/** The most jobs deep a view nests child jobs. */
export const MAX_DEPTH = 4;

/** One item of a view: the item, its depth, and the job whose events hold it. */
export type Shown = Item & { depth: number; in_job: string };

/** A job's progress, with its child jobs' items nested under the items that started them. */
export interface View {
  id: string;
  state: JobState["state"];
  /** The job's result, once it ended or said `done`. */
  result: Outcome | null;
  /** How many items stand in each state, across every nested job. */
  totals: Totals;
  /** The time left, when any of the run's steps has a history. */
  estimate: Estimate | null;
  /** The newest note about the job as a whole. */
  note: string | null;
  items: Shown[];
  /** Items past the bound, of a job or of the view. */
  over: number;
  /** Whether a job's events passed the bytes a fold reads. */
  truncated: boolean;
  /** Lines skipped as too long or not an event. */
  skipped_lines: number;
}

/** The fold of job `id` of `store`, standing at `state`, finished when it ended. */
export function foldOf(store: Store, id: string, state: JobState, now: number): Fold {
  const fold = Fold.read(join(store.dir(id), EVENTS));
  if (state.state === "exited") fold.finish(Number.isInteger(state.exit.code) ? state.exit.code : 1, state.exit.ended);
  else if (state.state === "lost") fold.finish(null, now);
  return fold;
}

/** The progress of job `id` of `store` as of `now`, with history from the project at `root`, or null without a record. */
export function view(store: Store, id: string, root: string | null, now = Date.now()): View | null {
  const record = store.record(id);
  if (!record) return null;
  const state = store.state(id, record);
  const top = foldOf(store, id, state, now);
  const v: View = {
    id,
    state: state.state,
    result: top.result,
    totals: noTotals(),
    estimate: null,
    note: top.note,
    items: [],
    over: 0,
    truncated: false,
    skipped_lines: 0,
  };
  const seen = new Set([id]);
  const nest = (job: string, fold: Fold, base: number, jobs: number): void => {
    v.over += fold.over;
    v.truncated ||= fold.truncated;
    v.skipped_lines += fold.longLines + fold.bad;
    for (const [depth, item] of fold.tree()) {
      if (v.items.length >= MAX_ITEMS) {
        v.over++;
        continue;
      }
      v.items.push({ ...item, depth: base + depth, in_job: job });
      const child = item.job;
      if (child === null || jobs + 1 >= MAX_DEPTH || seen.has(child)) continue;
      seen.add(child);
      const r = store.record(child);
      if (!r) continue;
      nest(child, foldOf(store, child, store.state(child, r), now), base + depth + 1, jobs + 1);
    }
  };
  nest(id, top, 0, 0);
  for (const s of v.items) count(v.totals, s);
  const steps = v.items.filter((s) => s.depth === 0);
  if (root !== null && steps.length > 0) {
    const left: Left[] = steps
      .filter((s) => s.state === "planned" || s.state === "waiting" || s.state === "running")
      .map((s) => ({ item: s.id, running_ms: s.state === "running" ? Math.max(0, now - (s.started ?? now)) : null }));
    const ids = steps.map((s) => s.id);
    v.estimate = estimate(medians(root, ids), ids, left);
  }
  return v;
}
