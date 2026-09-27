// Which proof programs `bun nv proofs --run` and `--verify` run, and recording the ones that ran.
//
// A program runs when the selection picks it (`tools/nv/select/`): its footprint holds a key the change
// since the store's tree moved, its definition changed (the program, its `.out`, `.in` and `.nvsr`, the `nvs.toml`
// beside it), it was never recorded, its last run was red or owed, or it is marked diverged. Every other
// program is reported green without a run. `--no-cache` runs every program in scope.
//
// Each program that runs is judged on the proof binary and recorded in a second run on the covws debug
// `nvs` (`recordingRun`), whose outcome is never the verdict: its coverage and footprint log go under the
// run's scratch directory, a batch of programs at a time so the disk holds one batch's counters. A
// recording run that ends differently from the judged run, with another exit status or other standard
// output, may have stopped short of code the judged run reached: the program is marked diverged
// (`SelectStore.markDiverged`), and the selection picks it every time until a recording run agrees
// again. A program that is skipped on this host holds its own file alone. The run, green or red, then
// moves the store's tree past the change (`advance`), with every other atom the change reached marked
// owed and a red program kept red. A run the sweep started leaves the tree to the sweep
// (`NO_ADVANCE_ENV`).
//
// A recording run costs a second process per program on a build many times slower than the judged one.
// That time buys a verdict taken on the binary a person ships, under the time limit written for it.

import { join } from "node:path";
import { type Graph, metadata } from "../keys/graph.ts";
import { buildCovws } from "../lib/covws.ts";
import { cargoLines } from "../lib/progress.ts";
import { unrecorded } from "../lib/reads.ts";
import { proofDef, proofId } from "../select/atoms.ts";
import { recordedIn } from "../select/extract.ts";
import { advance, chunks, fullChange, pool, Recorder } from "../select/record.ts";
import { type ChangeSet, computeChange, query, type Selection } from "../select/select.ts";
import { type Keyed, SelectStore, type Verdict } from "../select/store.ts";
import { type Binary, divergence, type Pass, type Program, recordingRun, recordName, type Result, type RunOptions, runPrograms, type What } from "./run.ts";

/** How many programs one recorded batch runs. */
const BATCH = 96;

async function selection(store: SelectStore, graph: Graph | null, programs: { path: string }[]): Promise<{ change: ChangeSet; sel: Selection }> {
  let change: ChangeSet;
  try {
    change = store.base() === null ? await fullChange() : await computeChange(store, { graph });
  } catch (e) {
    change = await fullChange(undefined, `the recorded tree could not be read: ${(e as Error).message.split("\n")[0]}`);
  }
  const sel = query(store, change, { discovered: programs.map((p) => proofId(p.path)) });
  return { change, sel };
}

/** A program whose recording run ended differently from its judged run, and why. */
export interface Diverged {
  path: string;
  why: string;
}

/**
 * Records one judged program: its recording run on `nvs` into `dir`, the keys that run left, and the
 * judged verdict. A recording run that diverged marks the program, and one that agreed clears its mark.
 * Returns why it diverged, or null.
 */
export async function recordProgram(rec: Recorder, nvs: string, dir: string, what: What, path: string, result: Result | undefined): Promise<string | null> {
  const id = proofId(path);
  const verdict: Verdict = !result || result.verdict === "fail" ? "red" : "green";
  // A program skipped on this host ran nothing; it is selected again when its own file changes.
  if (!result?.ran) {
    rec.record(id, proofDef(path), verdict, new Map([[`file:${path}`, ""]]));
    return null;
  }
  const recorded = await recordingRun(nvs, what, path, dir, rec.tmpEnv, result.ran);
  const got = recordedIn(dir).get(recordName(path));
  const keys: Keyed = got ? (await rec.extract(got, [nvs])).keys : new Map();
  if (keys.size === 0) keys.set(`file:${path}`, "");
  rec.record(id, proofDef(path), verdict, keys);
  const why = divergence(result.ran, recorded);
  if (why === null) rec.store.clearDiverged(id);
  else rec.store.markDiverged(id, why);
  return why;
}

/**
 * Runs the programs of `programs` the change selects, all of them with `all`, and records each.
 *
 * A recorded `bun nv proofs` notes only what its verdict reads: each judged program's own files, read
 * here. The selection, the recording runs and moving the store's tree run `unrecorded`, since each
 * program's footprint is its `proof:` atom's, taken on its recording run. A judged run writes no footprint
 * log for the same reason, so the programs' reads never land in the footprint of the check that ran them.
 */
export async function runSelected(bin: Binary, programs: Program[], opts: RunOptions, all: boolean): Promise<Pass & { ran: number; diverged: Diverged[] }> {
  const store = new SelectStore();
  const { graph, change, sel, rec } = await unrecorded(async () => {
    const graph = await metadata();
    const { change, sel } = await selection(store, graph, programs);
    return { graph, change, sel, rec: await Recorder.open(store, change.view, graph, "proofs") };
  });
  const chosen = programs.filter((p) => all || sel.selected.has(proofId(p.path)));
  const picked = new Set(chosen.map((p) => p.path));
  const unchanged = new Set(programs.filter((p) => !picked.has(p.path)).map((p) => p.path));
  const judged: RunOptions = { ...opts, unlogged: true };
  const results: Pass = { results: new Map(), width: 1, seconds: 0 };
  const ran = new Set<string>();
  const diverged: Diverged[] = [];
  try {
    // What is not run is reported first, in one pass with no programs to run.
    const quiet = await runPrograms(bin, programs.filter((p) => unchanged.has(p.path)), judged, unchanged);
    for (const [k, v] of quiet.results) results.results.set(k, v);
    // The recording build is the pipeline's debug build, which cargo brings up to date.
    const nvs = chosen.length > 0 ? (await unrecorded(() => buildCovws({ onLine: cargoLines("proofs: building the covws debug nvs") }))).nvs : "";
    for (const [n, batch] of chunks(chosen, BATCH).entries()) {
      // Every program of the batch is judged before any is recorded, so a judged run never shares the
      // machine with the slower recording runs.
      const pass = await runPrograms(bin, batch, judged);
      results.width = Math.max(results.width, pass.width);
      results.seconds += pass.seconds;
      const dir = join(rec.dir, "proofs", String(n));
      await unrecorded(() =>
        pool(batch, Math.max(1, pass.width), async ({ what, path }) => {
          const result = pass.results.get(`${what}:${path}`);
          if (result) results.results.set(`${what}:${path}`, result);
          const why = await recordProgram(rec, nvs, dir, what, path, result);
          if (why !== null) diverged.push({ path, why });
          ran.add(proofId(path));
        }),
      );
    }
    // Red or green, the tree moves; a red program stays selected as red.
    unrecorded(() => advance(store, change, sel, ran, graph));
  } finally {
    unrecorded(() => {
      rec.close();
      store.close();
    });
  }
  diverged.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
  return { ...results, ran: ran.size, diverged };
}
