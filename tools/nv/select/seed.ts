// `bun nv select --seed`: record every atom once, from nothing, so a selection has a footprint to read.
//
// It builds `covws` with its test executables, scans every Rust file into the store as the base tree's
// items, records each generated file's digest, and then runs every atom with recording on
// (`record.ts`):
//
// - the case trees, a batch of cases at a time through `recordCases`;
// - every proof program of the roster, through `runPrograms` with `NV_PROOF_RECORD` set;
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
import { namedBinary, RECORD_ENV, recordName, runPrograms } from "../proofs/run.ts";
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

interface Ctx {
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

async function seedProofs(ctx: Ctx, programs: { what: "examples" | "hostile"; path: string }[]): Promise<void> {
  const dir = join(ctx.r.dir, "proofs");
  const was = process.env[RECORD_ENV];
  const tmp = { TMP: process.env.TMP, TEMP: process.env.TEMP, TMPDIR: process.env.TMPDIR };
  process.env[RECORD_ENV] = dir;
  Object.assign(process.env, ctx.r.tmpEnv);
  let done = 0;
  try {
    for (const batch of chunks(programs, 96)) {
      rmSync(dir, { recursive: true, force: true });
      mkdirSync(dir, { recursive: true });
      const pass = await runPrograms(namedBinary(ctx.nvs), batch, { valgrind: false, strict: false });
      const recorded = recordedIn(dir);
      await pool(batch, ctx.jobs, async ({ what, path }) => {
        const got = recorded.get(recordName(path));
        const ext = got ? await ctx.r.extract(got, [ctx.nvs]) : null;
        const result = pass.results.get(`${what}:${path}`);
        const verdict: Verdict = !result || result.verdict === "fail" ? "red" : "green";
        ctx.r.record(proofId(path), proofDef(path, ctx.root), verdict, ext?.keys ?? new Map());
        ctx.tally("proof", verdict, ext);
      });
      done += batch.length;
      ctx.say(`select: proof programs ${done}/${programs.length}`);
    }
  } finally {
    if (was === undefined) delete process.env[RECORD_ENV];
    else process.env[RECORD_ENV] = was;
    for (const [k, v] of Object.entries(tmp)) {
      if (v === undefined) delete process.env[k];
      else process.env[k] = v;
    }
  }
}

async function seedTests(ctx: Ctx, exes: { pkg: string; t: TestExe }[]): Promise<void> {
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
    ctx.r.record(id, "", verdict, testKeys(pkg, ext, id.slice(5)));
    ctx.tally("test", verdict, ext);
    ctx.say(`select: test binaries ${++done}/${exes.length}${verdict === "red" ? ` (${id} red)` : ""}`);
  });
}

let wide: Set<string> | null = null;

/** A test binary's footprint: what it ran, and `tests:<package>`. A test added to its package cannot be
 * in any footprint yet, so every binary of the package holds that key, which an added test moves; a
 * binary that ran no instrumented code holds only this one. A binary `tools/data/impact-wide.txt` lists
 * opens files nothing records (`keys/escape.ts`), so it also holds `*`, which every change moves. */
export function testKeys(pkg: string, ext: Extracted | null, name?: string): Keyed {
  const keys: Keyed = new Map(ext?.keys ?? []);
  keys.set(testsKey(pkg), "");
  wide ??= allowedWide();
  if (name !== undefined && wide.has(name)) keys.set(WILD, "");
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

/** What a `bun nv` process's reads log says, as keys; `*` when it left no log, since a check that left no
 * record of its reads read anything it liked. */
export function nvKeys(log: string): Keyed {
  const reads = existsSync(log) ? readLog(log) : null;
  const keys: Keyed = new Map();
  if (!reads) keys.set("*", "");
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
