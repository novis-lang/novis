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
// (`judgeThenRecord`). Each judged verdict is kept in the store the moment it is final (`judgeKept`), and
// each recording counts the moment it is written, stamped with the tree (`store.ts`), so a run killed
// in either pool loses only the programs still running, and a program judged on this tree and not yet
// recorded is recorded without being judged again. A recording run that ends differently from the judged run, with another exit
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
// A bench is recorded the same way under its own atom, `bench:<path>` (`recordBench`), in one run at its
// smallest batch, and its verdict is its perf proof's growth. `runBenches` runs the benches a change
// selects. `bun nv affected --run` asks it, after `nv verify`, for the ones the change itself reached;
// `bun nv debt`, the floor check at each goal's end, asks it for the owed and red ones.
//
// The selection engine, the crate graph and the recorder are loaded by `engine`, through
// `loadUnrecorded`: they decide which programs run and what the store remembers, and none of them
// judges a program, so a change to them does not reach the `nv:` atom of every proofs check. This module,
// `run.ts` that judges and blesses, and the gate's modules stay in that footprint.

import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Options } from "../cmd/scaling.ts";
import type { Graph } from "../keys/graph.ts";
import { buildCovws } from "../lib/covws.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { read } from "./roster.ts";
import { cargoLines } from "../lib/progress.ts";
import { loadUnrecorded, unrecorded } from "../lib/reads.ts";
import type { Extracted } from "../select/extract.ts";
import type { Recorder } from "../select/record.ts";
import type { ChangeSet, Selected, Selection } from "../select/select.ts";
import { type Keyed, SelectStore, type Verdict } from "../select/store.ts";
import { progress } from "../lib/progress.ts";
import { type Binary, comparableRun, divergence,elidedOut, HANG_FACTOR, jobsFor, type Pass, type Program, recordingRun, recordName, type Result, type RunOptions, runPrograms, spawnProof, type What } from "./run.ts";

/** `// bench: iterations 200000` inside a bench program. */
const ITERATIONS_RE = /(?:\/\/|#)\s*bench:\s*iterations\s+([0-9_]+)/;
/** How long a bench's recording run may take: a minute on the optimized build, times the debug build's
 * slowdown. */
const BENCH_RECORD_MS = 60_000 * HANG_FACTOR;

async function load() {
  const [graph, atoms, extract, record, select] = await Promise.all([
    import("../keys/graph.ts"),
    import("../select/atoms.ts"),
    import("../select/extract.ts"),
    import("../select/record.ts"),
    import("../select/select.ts"),
  ]);
  const { advance, fullChange, pool, Recorder } = record;
  return { metadata: graph.metadata, benchDef: atoms.benchDef, benchFiles: atoms.benchFiles, benchId: atoms.benchId, proofDef: atoms.proofDef, proofId: atoms.proofId, recordedIn: extract.recordedIn, advance, fullChange, pool, Recorder, changedItems: select.changedItems, computeChange: select.computeChange, query: select.query };
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

/** One recorded program: its judged verdict, why its recording run diverged or null, and what that
 * run's footprint extracted, null for a program that ran nothing. */
export interface RecordedProgram {
  verdict: Verdict;
  why: string | null;
  ext: Extracted | null;
}

/**
 * Records one judged program of the tree at `root`: its recording run on `nvs` into `dir`, the keys
 * that run left, its two durations and the judged verdict. A recording run that diverged marks the
 * program, and one that agreed clears its mark. A program whose file is gone by now, renamed or deleted
 * after it was judged, is neither recorded nor marked: its recording run could only fail, and the next
 * selection drops its atom. Returns what was recorded, or null for a program that is gone.
 */
export async function recordProgram(rec: Recorder, nvs: string, dir: string, what: What, path: string, result: Result | undefined, root: string = ROOT): Promise<RecordedProgram | null> {
  if (!existsSync(abs(path, root))) return null;
  const { proofDef, proofId, recordedIn } = await engine();
  const id = proofId(path);
  const verdict: Verdict = !result || result.verdict === "fail" ? "red" : "green";
  // A program skipped on this host ran nothing; it is selected again when its own file changes.
  if (!result?.ran) {
    rec.record(id, proofDef(path, root), verdict, new Map([[`file:${path}`, ""]]));
    return { verdict, why: null, ext: null };
  }
  const recorded = await recordingRun(nvs, what, path, dir, rec.tmpEnv, result.ran);
  const got = recordedIn(dir).get(recordName(path));
  const ext = got ? await rec.extract(got, [nvs]) : null;
  const keys: Keyed = ext?.keys ?? new Map();
  if (keys.size === 0) keys.set(`file:${path}`, "");
  rec.record(id, proofDef(path, root), verdict, keys);
  rec.store.setDurations(id, result.ran.ms, recorded.ms);
  const why = divergence(comparableRun(what, path, result.ran), comparableRun(what, path, recorded), elidedOut(path, root));
  if (why === null) rec.store.clearDiverged(id);
  else rec.store.markDiverged(id, why);
  return { verdict, why, ext };
}

/**
 * Records one bench's footprint under `bench:<path>`, with `verdict` from its perf proof. The recording
 * run is one run of a copy of the bench on `nvs` at the smallest batch, `START` operations or its own N
 * when that is smaller: which code a bench reaches does not depend on how many times it runs. A bench
 * whose closing line cannot be rewritten runs at its N. The copy goes under `dir` and is deleted with it.
 * Returns why the recording run failed, or null; a run that failed records the bench's own file alone,
 * so it is selected again when that file changes.
 */
export async function recordBench(rec: Recorder, nvs: string, dir: string, path: string, verdict: Verdict): Promise<string | null> {
  const { benchDef, benchId, recordedIn } = await engine();
  const { copyBench, START, withBatch, writeCopy } = await import("../cmd/scaling.ts");
  const id = benchId(path);
  const source = read(path);
  const n = ITERATIONS_RE.exec(source);
  const iterations = n ? Number(n[1]!.replace(/_/g, "")) : 0;
  const root = join(dir, "copies", recordName(path));
  const copy = copyBench(path, root);
  const small = iterations > 0 ? withBatch(source, iterations, Math.min(START, iterations)) : null;
  if (small !== null) writeCopy(path, root, small);
  const records = join(dir, "benches");
  const ran = await spawnProof([nvs, "run", copy], copy, BENCH_RECORD_MS, { dir: records });
  const got = ran.code === 0 && !ran.timedOut ? recordedIn(records).get(recordName(copy)) : undefined;
  const keys: Keyed = got ? (await rec.extract(got, [nvs])).keys : new Map();
  keys.set(`file:${path}`, "");
  rec.record(id, benchDef(path), verdict, keys);
  rec.store.setDurations(id, 0, ran.ms);
  if (ran.timedOut) return `the recording run was still running after ${Math.round(ran.ms / 1000)}s`;
  return ran.code === 0 ? null : `the recording run exited ${ran.code}`;
}

/** A bench whose growth failed, and each way it failed. */
export interface RedBench {
  path: string;
  findings: string[];
}

/**
 * A bench a change reaches, and the changed items it executes: the changed keys its recorded footprint
 * holds, or that an earlier run owed it for. `items` is null for a bench that runs whatever is proven:
 * one never recorded, last red, owed for a reason no key gives, whose own files changed, or reached by a
 * change to the whole tree.
 */
export interface Reach {
  path: string;
  items: string[] | null;
}

/** How many benches that execute a changed item must show no degradation before the item is proven. */
export const PROVEN_BY = 2;

/**
 * `reach` in the order its benches run: each bench `items` cannot skip first, then the ones that execute
 * the most changed items, ties by path, so the same change always picks the same benches.
 */
export function benchOrder(reach: Reach[]): Reach[] {
  const weight = (r: Reach) => (r.items === null ? Infinity : r.items.length);
  return [...reach].sort((a, b) => weight(b) - weight(a) || (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}

/** What `planBenches` ran and skipped. */
export interface BenchPlan {
  /** The benches that ran, in the order they ran. */
  ran: string[];
  /** Each skipped bench, with the benches that proved its changed items. */
  skipped: Map<string, string[]>;
  /** The benches whose run showed a degradation. */
  degraded: string[];
  /** The benches neither run nor skipped, because `stop` said so first. */
  left: string[];
}

/**
 * Runs the benches of `reach` in `o.order`, `benchOrder` by default, `judge` saying of each whether it
 * showed a degradation, and skips each bench whose changed items are all proven: executed by `PROVEN_BY`
 * benches that showed none. A degradation unproves every item of the bench that showed it, so every bench
 * that executes one of them runs, a bench passed over before it included. `o.stop` is asked before each
 * bench starts, and once it says yes no other bench starts: each one not run is skipped when its items
 * are proven, and left otherwise.
 */
export async function planBenches(reach: Reach[], judge: (path: string) => Promise<boolean>, o: { order?: (reach: Reach[]) => Reach[]; stop?: () => boolean } = {}): Promise<BenchPlan> {
  const order = (o.order ?? benchOrder)(reach);
  const provers = new Map<string, string[]>();
  const degradedItems = new Set<string>();
  const ran: string[] = [];
  const done = new Set<string>();
  const degraded: string[] = [];
  const proven = (item: string) => !degradedItems.has(item) && (provers.get(item)?.length ?? 0) >= PROVEN_BY;
  const skips = (r: Reach) => r.items !== null && r.items.length > 0 && r.items.every(proven);
  for (;;) {
    const next = order.find((r) => !done.has(r.path) && !skips(r));
    if (next === undefined || o.stop?.()) break;
    done.add(next.path);
    ran.push(next.path);
    const bad = await judge(next.path);
    if (bad) degraded.push(next.path);
    for (const item of next.items ?? []) {
      if (bad) degradedItems.add(item);
      else provers.set(item, [...(provers.get(item) ?? []), next.path]);
    }
  }
  const skipped = new Map<string, string[]>();
  const left: string[] = [];
  for (const r of order) {
    if (done.has(r.path)) continue;
    if (skips(r)) skipped.set(r.path, [...new Set(r.items!.flatMap((item) => provers.get(item)!.slice(0, PROVEN_BY)))]);
    else left.push(r.path);
  }
  return { ran, skipped, degraded, left };
}

/**
 * Runs the benches the change since the store's tree selects: every one, or with `only` the selected ones
 * whose `bench:` id it names or whose selection it accepts. Each is judged by its perf proof's growth (`growthOf`) on the release
 * `nvs`, as `bun nv scaling --growth` judges it, and then recorded (`recordBench`) on the covws debug
 * `nvs`, with green when the growth found nothing or only what the bench's `// proof: gap` marker
 * records. The benches are judged in `planBenches`'s order, which skips a bench whose changed items are
 * already proven. Each bench is recorded right after it is judged, and one at a time, so no ramp shares
 * the machine with a debug run and a run stopped part of the way keeps every bench it finished. Each
 * growth is also kept the moment it is judged, so a bench judged on this tree by a run stopped before
 * its recording is recorded without being judged again. A skipped bench keeps its figure, and the tree
 * then moves past the change: a selected bench `only` leaves out is marked owed with the changed items
 * it was selected for, or kept red. With no bench to run, the store is not touched.
 *
 * `o.order` names the benches in the order they run, in place of `planBenches`'s own. No bench starts
 * once `Date.now()` reaches `o.deadline`, and each one not started is marked owed or kept red like a
 * bench `only` leaves out. Returns how many benches it chose and ran, each skipped one with the benches
 * that proved it, how many the deadline left, and the red ones, or why the release build failed.
 */
export async function runBenches(
  say: (line: string) => void = () => {},
  only?: Set<string> | ((s: Selected) => boolean),
  o: { order?: string[]; deadline?: number; onStart?: (id: string) => void } = {},
): Promise<{ reached: number; ran: number; skipped: Map<string, string[]>; left: number; red: RedBench[] } | string> {
  const { advance, benchFiles, benchId, changedItems, computeChange, fullChange, metadata, query, Recorder } = await engine();
  const { DEFAULT_WSL_NVS, growthOf } = await import("../cmd/scaling.ts");
  const { knownGap } = await import("./collect.ts");
  const { releaseBinary } = await import("./run.ts");
  const store = new SelectStore();
  try {
    const graph = await metadata();
    let change: ChangeSet;
    try {
      change = store.base() === null ? await fullChange() : await computeChange(store, { graph });
    } catch (e) {
      change = await fullChange(undefined, `the recorded tree could not be read: ${(e as Error).message.split("\n")[0]}`);
    }
    const all = benchFiles();
    const sel = query(store, change, { discovered: all.map(benchId) });
    const named = (s: Selected) => only === undefined || (typeof only === "function" ? only(s) : only.has(s.id));
    const chosen = all.filter((p) => {
      const s = sel.selected.get(benchId(p));
      return s !== undefined && named(s);
    });
    if (chosen.length === 0) return { reached: 0, ran: 0, skipped: new Map(), left: 0, red: [] };
    const built = await releaseBinary();
    if (typeof built === "string") return built;
    const rec = await Recorder.open(store, change.view, graph, "benches");
    const opts: Options = { nvs: built.path, reps: 2, wslNvs: DEFAULT_WSL_NVS, callgrind: false, scratch: join(rec.dir, "scaling") };
    const red: RedBench[] = [];
    try {
      const verdicts = new Map<string, Verdict>();
      const reach: Reach[] = chosen.map((path) => {
        const s = sel.selected.get(benchId(path))!;
        return { path, items: changedItems(s) };
      });
      const { nvs } = await buildCovws({ onLine: cargoLines("benches: building the covws debug nvs") });
      const ran = new Set<string>();
      // Each bench is recorded as soon as it is judged, so a run stopped part of the way keeps every
      // verdict it reached, and the next run selects only the benches still owed. Each growth is kept the
      // moment it is judged, so a run stopped during the recording does not judge it again on this tree.
      const key = store.judgedKey("bench growth");
      const rank = new Map((o.order ?? []).map((id, i) => [id, i]));
      const ranked = (r: Reach) => rank.get(benchId(r.path)) ?? rank.size;
      const order = o.order === undefined ? undefined : (rs: Reach[]) => [...rs].sort((a, b) => ranked(a) - ranked(b) || (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
      const stop = () => o.deadline !== undefined && Date.now() >= o.deadline;
      const plan = await planBenches(reach, async (path) => {
        const id = benchId(path);
        o.onStart?.(id);
        let judged = key === null ? null : store.judged<{ green: boolean; findings: string[] }>(id, key);
        say(`benches: ${verdicts.size + 1}/${chosen.length} ${judged ? "judged on this tree before" : "judging"} ${path}`);
        if (judged === null) {
          const g = await growthOf(path, opts, true);
          judged = { green: g.findings.length === 0 || knownGap(read(path)) !== null, findings: g.findings };
          if (key !== null) store.putJudged(id, key, judged);
        }
        verdicts.set(path, judged.green ? "green" : "red");
        if (!judged.green) red.push({ path, findings: judged.findings });
        say(`benches: ${verdicts.size}/${chosen.length} recording ${path}`);
        await recordBench(rec, nvs, join(rec.dir, "bench"), path, judged.green ? "green" : "red");
        ran.add(id);
        return !judged.green;
      }, { ...(order !== undefined && { order }), stop });
      // A skipped bench keeps its recorded footprint, so the tree moves past it too. Only a green bench
      // is skipped, and one an earlier run owed is green again. A bench the deadline left is not in
      // `ran`, so `advance` owes it.
      for (const path of plan.skipped.keys()) {
        store.setVerdict(benchId(path), "green");
        ran.add(benchId(path));
      }
      advance(store, change, sel, ran, graph);
      return { reached: chosen.length, ran: plan.ran.length, skipped: plan.skipped, left: plan.left.length, red };
    } finally {
      rec.close();
    }
  } finally {
    store.close();
  }
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
 * Judges `programs` on `bin` as `runPrograms` does, and keeps each verdict in `store` the moment it is
 * final, under the tree runs are taken at and the options it was judged with (`SelectStore.judgedKey`).
 * A program a run judged on this tree and was killed before recording is not judged again: its kept
 * verdict is used. Recording the program deletes it.
 */
export async function judgeKept(store: SelectStore, bin: Binary, programs: Program[], opts: RunOptions): Promise<Pass> {
  const { proofId } = await engine();
  const key = unrecorded(() => store.judgedKey(`proof ${opts.valgrind ? "valgrind" : "native"} ${opts.strict ? "strict" : "lenient"}`));
  const kept = new Map<string, Result>();
  const todo: Program[] = [];
  for (const p of programs) {
    const r = key === null ? null : unrecorded(() => store.judged<Result>(proofId(p.path), key));
    if (r) kept.set(`${p.what}:${p.path}`, r);
    else todo.push(p);
  }
  if (kept.size > 0) progress(`proofs: ${kept.size} program(s) were judged on this tree by a run that did not record them, so they are not judged again`);
  const keep = key === null ? {} : { onResult: (p: Program, r: Result) => unrecorded(() => store.putJudged(proofId(p.path), key, r)) };
  const pass = await runPrograms(bin, todo, { ...opts, ...keep });
  for (const [k, r] of kept) pass.results.set(k, r);
  return todo.length > 0 ? pass : { ...pass, width: jobsFor(programs.length) };
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
      (list) => judgeKept(store, bin, list, judged),
      (width, list, body) => unrecorded(() => pool(list, width, body)),
      async ({ what, path }, result) => {
        const why = (await recordProgram(rec, nvs, dir, what, path, result))?.why ?? null;
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
