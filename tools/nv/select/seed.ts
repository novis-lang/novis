// `bun nv select --seed`: record every atom once, from nothing, so a selection has a footprint to read.
//
// It builds `covws` with its test executables, scans every Rust file into the store as the base tree's
// items, records each generated file's digest, and then runs every atom with recording on
// (`record.ts`):
//
// - the case trees, a batch of cases at a time through `recordCases`;
// - every proof program of the roster, judged by `runPrograms` on the uninstrumented proof binary
//   (`target/proof`), then run once more on the covws debug `nvs` by `recordingRun` for its footprint;
//   a recording run that ends differently from the judged run marks the program diverged;
// - every Rust test executable of the build, one process each in its package directory, with
//   `LLVM_PROFILE_FILE` and `NVS_FOOTPRINT_LOG` inherited by every `nvs` it starts, and a compile cache
//   of its own that starts empty;
// - every `bun nv` command check of the live plan that reads and does not build, run, bench or write,
//   with `NV_READS_LOG` set, for its reads and the modules it loaded.
//
// Each atom's raw profiles and log are extracted and deleted as soon as its run ends, so the disk holds
// one batch's profiles at a time. The base tree becomes the tree as it stands.

import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { cpus } from "node:os";
import { join } from "node:path";
import { currentPlan } from "../lib/chain.ts";
import { buildCovws } from "../lib/covws.ts";
import { metadata } from "../keys/graph.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { readLog, readModules } from "../lib/reads.ts";
import type { TestExe } from "../driver/accept.ts";
import { divergence, proofBinary, recordingRun, recordName, runPrograms } from "../proofs/run.ts";
import { caseFiles, caseId, nvDef, nvId, proofDef, proofId, proofPrograms, testId } from "./atoms.ts";
import { type Extracted, recordedIn } from "./extract.ts";
import { allowedWide } from "../keys/escape.ts";
import { readsKeys, testsKey, WILD } from "./keys.ts";
import { advance, chunks, fullChange, pool, Recorder, recordCases } from "./record.ts";
import { type AtomKind, type Keyed, type SelectStore, type Verdict } from "./store.ts";

export { privateDir } from "./record.ts";

export interface SeedOptions {
  kinds?: AtomKind[];
  /** At most this many atoms of each kind, for a trial. */
  limit?: number;
  /** Skip every atom already recorded green on this platform: a seed that stopped goes on, and a red
   * atom runs again. */
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
const NOT_SEEDED = new Set(["bench", "db-matrix", "try", "verify", "loop", "release", "session", "splice", "selftest", "disk", "bg", "machine", "import", "relink", "select", "affected", "guard"]);

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
  nvs: string;
  jobs: number;
  say: (line: string) => void;
  tally: (kind: AtomKind, verdict: Verdict, ext: Extracted | null) => void;
  root: string;
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
  const recorded = new Set(opts.resume ? store.atoms().filter((a) => a.keys > 0 && a.verdict === "green").map((a) => a.id) : []);
  const limit = <T>(xs: T[], id: (x: T) => string) => {
    const left = xs.filter((x) => !recorded.has(id(x)));
    return opts.limit ? left.slice(0, opts.limit) : left;
  };
  const ctx: Ctx = { r, nvs: build.nvs, jobs, say, tally, root };
  try {
    if (kinds.includes("case")) {
      const cases = limit(caseFiles(root), caseId);
      const ran = await recordCases(r, build.nvs, cases, { jobs, onBatch: (done, total) => say(`select: cases ${done}/${total}`), root });
      for (const v of ran.verdicts.values()) tally("case", v, null);
    }
    if (kinds.includes("proof")) await seedProofs(ctx, limit(await proofPrograms(build.nvs), (p) => proofId(p.path)));
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

/** Judges every program of `programs` on the proof binary, records each one's footprint on the covws
 * `nvs`, and marks a program whose two runs ended differently diverged. `bun nv select --full` runs it
 * too. */
export async function seedProofs(ctx: Ctx, programs: { what: "examples" | "hostile"; path: string }[]): Promise<void> {
  if (programs.length === 0) return;
  const bin = await proofBinary();
  if (typeof bin === "string") throw new Error(`select: no proof binary to judge the proof programs on: ${bin}`);
  const dir = join(ctx.r.dir, "proofs");
  let done = 0;
  for (const batch of chunks(programs, 96)) {
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
    // Every program of the batch is judged before any is recorded, so a judged run never shares the
    // machine with the slower recording runs.
    const pass = await runPrograms(bin, batch, { valgrind: false, strict: false });
    await pool(batch, ctx.jobs, async ({ what, path }) => {
      const id = proofId(path);
      const result = pass.results.get(`${what}:${path}`);
      const verdict: Verdict = !result || result.verdict === "fail" ? "red" : "green";
      // A program skipped on this host ran nothing; it is selected again when its own file changes.
      if (!result?.ran) {
        ctx.r.record(id, proofDef(path, ctx.root), verdict, new Map([[`file:${path}`, ""]]));
        ctx.tally("proof", verdict, null);
        return;
      }
      const recorded = await recordingRun(ctx.nvs, what, path, dir, ctx.r.tmpEnv, result.ran);
      const got = recordedIn(dir).get(recordName(path));
      const ext = got ? await ctx.r.extract(got, [ctx.nvs]) : null;
      const keys: Keyed = ext?.keys ?? new Map();
      if (keys.size === 0) keys.set(`file:${path}`, "");
      ctx.r.record(id, proofDef(path, ctx.root), verdict, keys);
      ctx.tally("proof", verdict, ext);
      const why = divergence(result.ran, recorded);
      if (why === null) ctx.r.store.clearDiverged(id);
      else ctx.r.store.markDiverged(id, why);
    });
    done += batch.length;
    ctx.say(`select: proof programs ${done}/${programs.length}`);
  }
}

/** Runs every test executable of `exes` whole, in its package directory, and records each one. `bun nv
 * select --full` runs it too. */
export async function seedTests(ctx: Ctx, exes: { pkg: string; t: TestExe }[]): Promise<void> {
  const width = Math.max(1, Math.min(4, Math.floor(ctx.jobs / 2)));
  const threads = String(Math.max(1, Math.floor(cpus().length / width)));
  let done = 0;
  await pool(exes, width, async ({ pkg, t }) => {
    const id = testId(pkg, t);
    const name = recordName(id);
    // The binary's own compile cache starts empty, so the first compile of each program runs whole.
    // `NOVIS_NO_FILE_CACHE` is not set: the binaries that test the cache would test nothing.
    const env = { ...ctx.r.env(name), ...(await ctx.r.cacheDir(name)), CARGO_MANIFEST_DIR: t.dir, RUST_TEST_THREADS: threads, NO_COLOR: "1" };
    const p = await run([t.exe], { cwd: t.dir, env, timeoutMs: 3_600_000, reap: true });
    const ext = await ctx.r.keysOf(name, [t.exe, ctx.nvs]);
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
