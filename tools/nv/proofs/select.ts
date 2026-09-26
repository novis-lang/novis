// Which proof programs `bun nv proofs --run` and `--verify` run, and recording the ones that ran.
//
// A program runs when the selection picks it (`tools/nv/select/`): its footprint holds a key the change
// since the store's tree moved, its definition changed (the program, its `.out` and `.in`, the `nvs.toml`
// beside it), it was never recorded, or its last run was red or owed. Every other program is reported
// green without a run. `--no-cache` runs every program in scope.
//
// Each program that runs is recorded on the binary it ran on: its coverage and footprint log go under
// the run's scratch directory through `NV_PROOF_RECORD`, a batch of programs at a time so the disk holds
// one batch's counters, and a program that is skipped on this host holds its own file alone. A run with
// no failure then moves the store's tree past the change (`advance`), with every other atom the change
// reached marked owed.

import { join } from "node:path";
import { type Graph, metadata } from "../keys/graph.ts";
import { proofDef, proofId } from "../select/atoms.ts";
import { recordedIn } from "../select/extract.ts";
import { advance, chunks, fullChange, pool, Recorder } from "../select/record.ts";
import { type ChangeSet, computeChange, query, type Selection } from "../select/select.ts";
import { type Keyed, SelectStore, type Verdict } from "../select/store.ts";
import { type Binary, type Pass, RECORD_ENV, recordName, type RunOptions, runPrograms, type What } from "./run.ts";

/** How many programs one recorded batch runs. */
const BATCH = 96;

async function selection(store: SelectStore, graph: Graph | null, programs: { path: string }[]): Promise<{ change: ChangeSet; sel: Selection }> {
  let change: ChangeSet;
  try {
    change = store.base() === null ? await fullChange() : await computeChange(store, { graph });
  } catch (e) {
    change = await fullChange(undefined, `the recorded tree could not be read: ${(e as Error).message.split("\n")[0]}`);
  }
  const sel = query(store, change, { discovered: programs.map((p) => ({ id: proofId(p.path), def: proofDef(p.path) })) });
  return { change, sel };
}

/** Runs the programs of `programs` the change selects, all of them with `all`, and records each. */
export async function runSelected(bin: Binary, programs: { what: What; path: string }[], opts: RunOptions, all: boolean): Promise<Pass & { ran: number }> {
  const store = new SelectStore();
  const graph = await metadata();
  const { change, sel } = await selection(store, graph, programs);
  const chosen = programs.filter((p) => all || sel.selected.has(proofId(p.path)));
  const picked = new Set(chosen.map((p) => p.path));
  const unchanged = new Set(programs.filter((p) => !picked.has(p.path)).map((p) => p.path));
  const rec = await Recorder.open(store, change.view, graph, "proofs");
  const dir = join(rec.dir, "proofs");
  const saved = { [RECORD_ENV]: process.env[RECORD_ENV], TMP: process.env.TMP, TEMP: process.env.TEMP, TMPDIR: process.env.TMPDIR };
  process.env[RECORD_ENV] = dir;
  Object.assign(process.env, rec.tmpEnv);
  const results: Pass = { results: new Map(), width: 1, seconds: 0 };
  const ran = new Set<string>();
  let failed = false;
  try {
    // What is not run is reported first, in one pass with no programs to run.
    const quiet = await runPrograms(bin, programs.filter((p) => unchanged.has(p.path)), opts, unchanged);
    for (const [k, v] of quiet.results) results.results.set(k, v);
    for (const batch of chunks(chosen, BATCH)) {
      const pass = await runPrograms(bin, batch, opts);
      results.width = Math.max(results.width, pass.width);
      results.seconds += pass.seconds;
      const recorded = recordedIn(dir);
      await pool(batch, Math.max(1, pass.width), async ({ what, path }) => {
        const result = pass.results.get(`${what}:${path}`);
        if (result) results.results.set(`${what}:${path}`, result);
        const got = recorded.get(recordName(path));
        const keys: Keyed = got ? (await rec.extract(got, [bin.path])).keys : new Map();
        // A program skipped on this host ran nothing; it is selected again when its own file changes.
        if (keys.size === 0) keys.set(`file:${path}`, "");
        const verdict: Verdict = !result || result.verdict === "fail" ? "red" : "green";
        if (verdict === "red") failed = true;
        rec.record(proofId(path), proofDef(path), verdict, keys);
        ran.add(proofId(path));
      });
    }
    if (!failed) advance(store, change, sel, ran, graph);
  } finally {
    for (const [k, v] of Object.entries(saved)) {
      if (v === undefined) delete process.env[k];
      else process.env[k] = v;
    }
    rec.close();
    store.close();
  }
  return { ...results, ran: ran.size };
}
