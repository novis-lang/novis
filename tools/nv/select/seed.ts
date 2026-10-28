// `bun nv select --seed`: record every atom once, from nothing, so a selection has a footprint to read.
//
// It builds `covws` with its test executables, scans every Rust file into the store as the base tree's
// items, records each generated file's digest, and then runs every atom with recording on
// (`record.ts`):
//
// - the case trees, a batch of cases at a time through `recordCases`;
// - every proof program of the roster, judged by `judgeKept` on the uninstrumented proof binary
//   (`target/proof`), then run once more on the covws debug `nvs` by `recordProgram` for its footprint;
//   a recording run that ends differently from the judged run marks the program diverged, and a program
//   whose file is gone by then is not recorded;
// - every Rust test executable of the build, one process each in its package directory, with
//   `LLVM_PROFILE_FILE` and `NVS_FOOTPRINT_LOG` inherited by every `nvs` it starts, and a compile cache
//   of its own that starts empty;
// - every `bun nv` command check of the live plan that reads and does not build, run, bench or write,
//   with `NV_READS_LOG` set, for its reads and the modules it loaded.
//
// Each atom's raw profiles and log are extracted and deleted as soon as its run ends, so the disk holds
// one batch of cases' profiles, or those of the runs in flight, at a time. The base tree becomes the
// tree as it stands.

import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { cpus } from "node:os";
import { join } from "node:path";
import { currentPlan } from "../lib/chain.ts";
import { buildCovws } from "../lib/covws.ts";
import { metadata } from "../keys/graph.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { readLog, readModules } from "../lib/reads.ts";
import type { TestExe } from "../accept/accept.ts";
import { proofBinary, recordName } from "../proofs/run.ts";
import { judgeKept, recordProgram } from "../proofs/select.ts";
import { caseFiles, caseId, nvDef, nvId, proofId, proofPrograms, testId } from "./atoms.ts";
import type { Extracted } from "./extract.ts";
import { allowedWide } from "../keys/escape.ts";
import { readsKeys, testsKey, WILD } from "./keys.ts";
import { advance, fullChange, pool, Recorder, recordCases } from "./record.ts";
import { treeSig } from "./change.ts";
import { runTree } from "./select.ts";
import { type AtomKind, type Keyed, type SelectStore, type Verdict } from "./store.ts";

export { privateDir } from "./record.ts";

export interface SeedOptions {
  kinds?: AtomKind[];
  /** At most this many atoms of each kind, for a trial. */
  limit?: number;
  /** Skip every atom whose last run on this platform was green, recorded since the base or on the tree
   * as it is now: a seed that stopped goes on, and a red atom runs again. */
  resume?: boolean;
  jobs?: number;
  say?: (line: string) => void;
  root?: string;
}

export interface KindReport {
  run: number;
  green: number;
  red: number;
  processes: number;
  executed: number;
  unmapped: number;
}

export interface SeedReport {
  base: string;
  kinds: Partial<Record<AtomKind, KindReport>>;
  seconds: number;
}

/** The `bun nv` subcommands a seed never runs: each builds, runs programs, benches, or writes the tree. */
const NOT_SEEDED = new Set(["bench", "db-matrix", "try", "verify", "loop", "release", "session", "splice", "selftest", "disk", "machine", "relink", "vsix", "select", "affected", "guard"]);

/** Whether a `bun nv` check only reads: a seed runs it for what it reads. */
export function seedable(argv: string[]): boolean {
  const sub = argv[2] ?? "";
  if (NOT_SEEDED.has(sub)) return false;
  if (sub === "proofs" && argv.some((a) => ["--verify", "--run", "--only", "--record-perf", "--bless", "--perf-report"].includes(a))) return false;
  if (sub === "rules" && argv.includes("--render")) return false;
  if (sub === "render" && !argv.includes("--check")) return false;
  return true;
}

/** The labels a `nvs test` report names as failed; a skipped case is not red. */
export function failedLabels(out: string): Set<string> {
  const failed = new Set<string>();
  for (const line of out.split("\n")) {
    const m = /^FAIL (.+?)\r?$/.exec(line);
    if (m) failed.add(m[1]!.replace(/\\/g, "/").replace(/^\.\//, ""));
  }
  return failed;
}

/** What the runners below share: the recorder, the covws `nvs`, and how many runs go at once. */
export interface Ctx {
  r: Recorder;
  /** The `nvs` a recorded program runs: the run's pinned copy (`Recorder.pin`). */
  nvs: string;
  /** The covws build's own `nvs`, which a test binary starts by the path it was compiled with. */
  built: string;
  jobs: number;
  say: (line: string) => void;
  tally: (kind: AtomKind, verdict: Verdict, ext: Extracted | null) => void;
  root: string;
  /** Told of each atom a runner starts on its own; an atom of a batch is not told. */
  starting?: (id: string) => void;
}

export async function seed(store: SelectStore, opts: SeedOptions = {}): Promise<SeedReport> {
  const root = opts.root ?? ROOT;
  const say = opts.say ?? ((line: string) => console.log(line));
  const kinds = opts.kinds ?? ["case", "proof", "test", "nv"];
  const jobs = opts.jobs ?? Math.max(2, Math.floor(cpus().length / 2));
  const started = performance.now();

  say("select: building covws");
  const build = await buildCovws({ tests: kinds.includes("test") });
  say("select: scanning every Rust file");
  const change = await fullChange(root, "a seed");
  const here = treeSig(change.tree!);
  store.runAt(here, () => runTree(change.tree!, change.view, root));
  const graph = await metadata();
  const r = await Recorder.open(store, change.view, graph, "seed", { root, say });
  store.setMeta(`build:${store.platform}`, build.nvs);

  const report: SeedReport = { base: change.tree!.commit, kinds: {}, seconds: 0 };
  const tally = (kind: AtomKind, verdict: Verdict, ext: Extracted | null) => {
    const k = (report.kinds[kind] ??= { run: 0, green: 0, red: 0, processes: 0, executed: 0, unmapped: 0 });
    k.run++;
    if (verdict === "red") k.red++;
    else k.green++;
    if (ext) {
      k.processes += ext.processes;
      k.executed += ext.executed;
      k.unmapped += ext.unmapped;
    }
  };
  // A green atom stamped with a tree keeps its stamp, so the selection reads it against the change since
  // that tree; one with no stamp holds on the base's tree, so it is skipped only when that is this one.
  const base = store.base();
  const atBase = base !== null && treeSig({ commit: base, overlay: store.overlay() }) === here;
  const stamps = store.stamps();
  const recorded = new Set(opts.resume ? store.atoms().filter((a) => a.keys > 0 && a.verdict === "green" && (atBase || stamps.has(a.id))).map((a) => a.id) : []);
  const limit = <T>(xs: T[], id: (x: T) => string) => {
    const left = xs.filter((x) => !recorded.has(id(x)));
    return opts.limit ? left.slice(0, opts.limit) : left;
  };
  const ctx: Ctx = { r, nvs: r.pin(build.nvs), built: build.nvs, jobs, say, tally, root };
  try {
    if (kinds.includes("case")) {
      const cases = limit(caseFiles(root), caseId);
      const ran = await recordCases(r, ctx.nvs, cases, { jobs, onBatch: (done, total) => say(`select: cases ${done}/${total}`), root });
      for (const v of ran.verdicts.values()) tally("case", v, null);
    }
    if (kinds.includes("proof")) await seedProofs(ctx, limit(await proofPrograms(ctx.nvs), (p) => proofId(p.path)));
    if (kinds.includes("test")) await seedTests(ctx, limit([...build.tests].flatMap(([pkg, ts]) => ts.map((t) => ({ pkg, t }))), (x) => testId(x.pkg, x.t)));
    if (kinds.includes("nv")) await seedNv(ctx, limit(nvChecks(root), (c) => nvId(c.id)));
    // Every atom of the kinds seeded ran; the store's items and tree are the ones scanned above.
    advance(store, change, null, new Set(), graph, root);
  } finally {
    r.close();
  }
  report.seconds = Math.round((performance.now() - started) / 1000);
  return report;
}

/** Judges every program of `programs` on the proof binary in one pool, then records each judged one
 * through `recordProgram`, the way `bun nv proofs` records one, in a second pool of `ctx.jobs`. `bun nv
 * select --full` runs it too. `o.stop` is asked before each program's judged run and before each
 * recording run starts; a program it stops before its judged run is not recorded.
 *
 * Neither phase is cut into batches: a batch waits for its slowest program, and a recording run takes
 * from 0.1 s to 50 s, so batches of eight left most of the pool idle and took 46 minutes for work one
 * pool does in 17. Each program's raw profiles are deleted once they are read (`extract`), so the
 * record directory holds only the runs in flight. What it spends: the judged result of every program,
 * its output included, is held until the program is recorded.
 */
export async function seedProofs(ctx: Ctx, programs: { what: "examples" | "hostile"; path: string }[], o: { stop?: () => boolean } = {}): Promise<void> {
  if (programs.length === 0 || o.stop?.()) return;
  const bin = await proofBinary();
  if (typeof bin === "string") throw new Error(`select: no proof binary to judge the proof programs on: ${bin}`);
  const dir = join(ctx.r.dir, "proofs");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  // Every program is judged before any is recorded, so a judged run never shares the machine with the
  // slower recording runs.
  const pass = await judgeKept(ctx.r.store, bin, programs, { valgrind: false, strict: false, ...(o.stop && { stop: o.stop }) });
  let done = 0;
  await pool(programs, ctx.jobs, async ({ what, path }) => {
    const result = pass.results.get(`${what}:${path}`);
    if (result === undefined || o.stop?.()) return;
    const recorded = await recordProgram(ctx.r, ctx.nvs, dir, what, path, result, ctx.root);
    if (recorded) ctx.tally("proof", recorded.verdict, recorded.ext);
    done++;
    if (done % ctx.jobs === 0 || done === programs.length) ctx.say(`select: proof programs ${done}/${programs.length}`);
  });
}

/** Runs every test executable of `exes` whole, in its package directory, and records each one. `bun nv
 * select --full` runs it too. `stop` is asked before each executable starts, and one it says yes to is
 * not started. */
export async function seedTests(ctx: Ctx, exes: { pkg: string; t: TestExe }[], stop: () => boolean = () => false): Promise<void> {
  const width = Math.max(1, Math.min(4, Math.floor(ctx.jobs / 2)));
  const threads = String(Math.max(1, Math.floor(cpus().length / width)));
  let done = 0;
  await pool(exes, width, async ({ pkg, t }) => {
    if (stop()) return;
    const id = testId(pkg, t);
    ctx.starting?.(id);
    const name = recordName(id);
    // The binary's own compile cache starts empty, so the first compile of each program runs whole.
    // `NOVIS_NO_FILE_CACHE` is not set: the binaries that test the cache would test nothing.
    const env = { ...ctx.r.env(name), ...(await ctx.r.cacheDir(name)), CARGO_MANIFEST_DIR: t.dir, RUST_TEST_THREADS: threads, NO_COLOR: "1" };
    const p = await run([t.exe], { cwd: t.dir, env, timeoutMs: 3_600_000, reap: true });
    const ext = await ctx.r.keysOf(name, [t.exe, ctx.built]);
    const verdict: Verdict = p.code === 0 && ext ? "green" : "red";
    ctx.r.record(id, "", verdict, testKeys(ext, id.slice(5)));
    ctx.tally("test", verdict, ext);
    ctx.say(`select: test binaries ${++done}/${exes.length}${verdict === "red" ? ` (${id} red)` : ""}`);
  });
}

let wide: Set<string> | null = null;

/** Test binary `name`'s footprint (`<package> <kind> <target>`): what it ran, and its own `tests:` key.
 * A test added to a file compiled into the binary cannot be in any footprint yet, so the binary holds
 * that key, which the added test moves; a binary that ran no instrumented code holds only this one. A
 * binary `tools/data/impact-wide.txt` lists opens files nothing records (`keys/escape.ts`), so it also
 * holds `*`, which every change moves. */
export function testKeys(ext: Extracted | null, name: string): Keyed {
  const keys: Keyed = new Map(ext?.keys ?? []);
  keys.set(testsKey(name), "");
  wide ??= allowedWide();
  if (wide.has(name)) keys.set(WILD, "");
  return keys;
}

/** The `bun nv` command checks a seed runs: the side goal's plan when `NOVIS_SIDE_GOAL` names one, else
 * the live goal's. */
export function nvChecks(root: string = ROOT): { id: string; argv: string[]; cwd: string }[] {
  let checks: { id: string; kind: string; argv?: string[]; cwd?: string }[];
  try {
    checks = (currentPlan(root)?.goal.checks ?? []) as typeof checks;
  } catch {
    return [];
  }
  return checks
    .filter((c) => c.kind === "command" && (c.argv ?? []).slice(0, 2).join(" ") === "bun nv" && seedable(c.argv!))
    .map((c) => ({ id: c.id, argv: c.argv!, cwd: c.cwd ?? "." }));
}

/** What a `bun nv` process's reads log says, as keys, with only `part` of its parts when one is named
 * (`readLog`); `*` when it left no log, since a check that left no record of its reads read anything it
 * liked. */
export function nvKeys(log: string, part?: string): Keyed {
  const reads = existsSync(log) ? readLog(log, part) : null;
  const keys: Keyed = new Map();
  if (!reads) {
    console.error(`select: ${log} holds no reads, so its check depends on everything until a run of it leaves its reads`);
    keys.set("*", "");
  }
  else for (const k of readsKeys(reads, readModules(log))) keys.set(k, "");
  return keys;
}

async function seedNv(ctx: Ctx, checks: { id: string; argv: string[]; cwd: string }[]): Promise<void> {
  const dir = join(ctx.r.dir, "nv");
  mkdirSync(dir, { recursive: true });
  let done = 0;
  await pool(checks, Math.min(4, ctx.jobs), async (c) => {
    const log = join(dir, `${recordName(c.id)}.reads`);
    const p = await run(c.argv, { cwd: join(ctx.root, c.cwd), env: { ...ctx.r.tmpEnv, NO_COLOR: "1", NV_READS_LOG: log }, timeoutMs: 900_000, reap: true });
    const keys = nvKeys(log);
    rmSync(log, { force: true });
    const verdict: Verdict = p.code === 0 ? "green" : "red";
    ctx.r.record(nvId(c.id), nvDef(c.argv, c.cwd), verdict, keys);
    ctx.tally("nv", verdict, null);
    ctx.say(`select: bun nv checks ${++done}/${checks.length}`);
  });
}
