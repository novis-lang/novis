// Which proof programs `bun nv proofs --run` and `--verify` run, and recording the ones that ran.
//
// A program runs when the selection picks it (`tools/nv/select/`): its footprint holds a key the change
// since the store's tree moved, its definition changed (the program, its `.out`, `.in` and `.nvsr`, the `nvs.toml`
// beside it), it was never recorded, its last run was red or owed, or it is marked diverged. Every other
// program is reported green without a run. `--no-cache` runs every program in scope.
//
// Each program that runs is judged on the proof binary and recorded in a second run on the covws debug
// `nvs` (`recordingRun`), whose outcome is never the verdict: its coverage and footprint log go under the
// run's scratch directory, and are turned into keys and deleted as soon as that run ends, so the disk
// holds the counters of at most one pool's width of programs. Every program is judged in one pool before
// any is recorded in a second, each pool longest first by the time the store noted for that kind of run
// (`judgeThenRecord`). A recording run that ends differently from the judged run, with another exit
// status or other standard output, may have stopped short of code the judged run reached: the program is marked diverged
// (`SelectStore.markDiverged`), and the selection picks it every time until a recording run agrees
// again. A program that is skipped on this host holds its own file alone. The run, green or red, then
// moves the store's tree past the change (`advance`), with every other atom the change reached marked
// owed and a red program kept red. A run the sweep started leaves the tree to the sweep
// (`NO_ADVANCE_ENV`).
//
// A recording run costs a second process per program on a build many times slower than the judged one.
// That time buys a verdict taken on the binary a person ships, under the time limit written for it.
//
// The selection engine, the crate graph and the recorder are loaded by `engine`, through
// `loadUnrecorded`: they decide which programs run and what the store remembers, and none of them
// judges a program, so a change to them does not reach the `nv:` atom of every proofs check. This module,
// `run.ts` that judges and blesses, and the gate's modules stay in that footprint.

import { join } from "node:path";
import type { Graph } from "../keys/graph.ts";
import { buildCovws } from "../lib/covws.ts";
import { cargoLines } from "../lib/progress.ts";
import { loadUnrecorded, unrecorded } from "../lib/reads.ts";
import type { Recorder } from "../select/record.ts";
import type { ChangeSet, Selection } from "../select/select.ts";
import { type Keyed, SelectStore, type Verdict } from "../select/store.ts";
import { progress } from "../lib/progress.ts";
import { type Binary, divergence, jobsFor, type Pass, type Program, recordingRun, recordName, type Result, type RunOptions, runPrograms, type What } from "./run.ts";

async function load() {
  const [graph, atoms, extract, record, select] = await Promise.all([
    import("../keys/graph.ts"),
    import("../select/atoms.ts"),
    import("../select/extract.ts"),
    import("../select/record.ts"),
    import("../select/select.ts"),
  ]);
  const { advance, fullChange, pool, Recorder } = record;
  return { metadata: graph.metadata, proofDef: atoms.proofDef, proofId: atoms.proofId, recordedIn: extract.recordedIn, advance, fullChange, pool, Recorder, computeChange: select.computeChange, query: select.query };
}

let loaded: ReturnType<typeof load> | undefined;

/** The selection engine and the recorder, loaded once and `loadUnrecorded`. */
function engine(): ReturnType<typeof load> {
  loaded ??= loadUnrecorded(load);
  return loaded;
}

async function selection(store: SelectStore, graph: Graph | null, programs: { path: string }[]): Promise<{ change: ChangeSet; sel: Selection }> {
  const { computeChange, fullChange, proofId, query } = await engine();
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
  const { proofDef, proofId, recordedIn } = await engine();
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
  rec.store.setDurations(id, result.ran.ms, recorded.ms);
  const why = divergence(result.ran, recorded);
  if (why === null) rec.store.clearDiverged(id);
  else rec.store.markDiverged(id, why);
  return why;
}

/**
 * `programs` in the order a pool should start them, longest first so that no long program starts last
 * and runs alone at the end. A program with a known time (`ms`) goes by it; one with none goes before
 * every known one, larger `prior` first, since it may be the longest of all. Ties go by path.
 */
export function longestFirst<P extends { path: string }>(programs: P[], ms: (p: P) => number | undefined, prior: (p: P) => number): P[] {
  const key = programs.map((p) => {
    const t = ms(p);
    return { p, known: t !== undefined && t > 0, weight: t !== undefined && t > 0 ? t : prior(p) };
  });
  key.sort((a, b) => (a.known !== b.known ? (a.known ? 1 : -1) : b.weight - a.weight || (a.p.path < b.p.path ? -1 : a.p.path > b.p.path ? 1 : 0)));
  return key.map((k) => k.p);
}

/**
 * Judges every program of `programs` in one pool (`judge`), and only then records each in a second pool
 * as wide as the first (`pool`, running `record`). No judged run ever shares the machine with a
 * recording run: a recording run on the debug build is many times slower, and an attack judged beside
 * one could run out of its time limit for the load alone.
 *
 * Each pool starts its programs longest first (`longestFirst`) by the time noted for its own kind of
 * run: the judged time for judging, the recorded time for recording. A program never judged here goes
 * first, an attack before an example, since attacks carry the long limits. A program never recorded goes
 * first in the recording pool, by the time this pass judged it in.
 */
export async function judgeThenRecord<P extends Program>(
  programs: P[],
  took: (p: P) => { judged: number; recorded: number } | undefined,
  judge: (ordered: P[]) => Promise<Pass>,
  pool: (width: number, ordered: P[], body: (p: P) => Promise<void>) => Promise<void>,
  record: (p: P, result: Result | undefined) => Promise<void>,
): Promise<Pass> {
  if (programs.length === 0) return { results: new Map(), width: 1, seconds: 0 };
  const pass = await judge(longestFirst(programs, (p) => took(p)?.judged, (p) => (p.what === "hostile" ? 1 : 0)));
  const resultOf = (p: P) => pass.results.get(`${p.what}:${p.path}`);
  const ordered = longestFirst(programs, (p) => took(p)?.recorded, (p) => resultOf(p)?.ran?.ms ?? 0);
  await pool(Math.max(1, pass.width), ordered, (p) => record(p, resultOf(p)));
  return pass;
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
  const { advance, metadata, pool, proofId, Recorder } = await engine();
  const store = new SelectStore();
  const { graph, change, sel, rec } = await unrecorded(async () => {
    const graph = await metadata();
    const { change, sel } = await selection(store, graph, programs);
    return { graph, change, sel, rec: await Recorder.open(store, change.view, graph, "proofs", { extractWidth: jobsFor(programs.length) }) };
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
    const took = unrecorded(() => store.durations("proof"));
    const dir = join(rec.dir, "proofs");
    let recorded = 0;
    const pass = await judgeThenRecord(
      chosen,
      (p) => took.get(proofId(p.path)),
      (list) => runPrograms(bin, list, judged),
      (width, list, body) => unrecorded(() => pool(list, width, body)),
      async ({ what, path }, result) => {
        const why = await recordProgram(rec, nvs, dir, what, path, result);
        if (why !== null) diverged.push({ path, why });
        ran.add(proofId(path));
        progress(`proofs: ${++recorded}/${chosen.length} programs recorded`);
      },
    );
    for (const [k, v] of pass.results) results.results.set(k, v);
    results.width = pass.width;
    results.seconds = pass.seconds;
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
