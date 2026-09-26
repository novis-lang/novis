// `bun nv select --seed`: record every atom once, from nothing, so a selection has a footprint to read.
//
// It builds `covws` with its test executables, scans every Rust file into the store as the base tree's
// items, records each generated file's digest, and then runs every atom with recording on:
//
// - the case trees, a batch of cases at a time through `nvs test --cases <list> --record <dir>`; the
//   `nvs test` parent is recorded too, and what it ran is part of every case of its batch;
// - every proof program of the roster, through `runPrograms` with `NV_PROOF_RECORD` set;
// - every Rust test executable of the build, one process each in its package directory, with
//   `LLVM_PROFILE_FILE` and `NVS_FOOTPRINT_LOG` inherited by every `nvs` it starts, and a compile cache
//   of its own that starts empty;
// - every `bun nv` command check of the live plan that reads and does not build, run, bench or write,
//   with `NV_READS_LOG` set, for its reads and the modules it loaded.
//
// Each atom's raw profiles and log are extracted and deleted as soon as its run ends, so the disk holds
// one batch's profiles at a time. The base tree becomes the commit `HEAD` names.
//
// Every scratch file goes under `.agent-tmp/select-rec/`, and every process started gets its temporary
// directory there too, which is deleted at the end.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { cpus } from "node:os";
import { join } from "node:path";
import { goalPlan } from "../lib/chain.ts";
import { buildCovws, type CovwsBuild } from "../lib/covws.ts";
import { metadata } from "../keys/graph.ts";
import { scanItems } from "../keys/scan.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { readLog, readModules } from "../lib/reads.ts";
import { namedBinary, RECORD_ENV, recordName, runPrograms } from "../proofs/run.ts";
import { caseDef, caseFiles, caseId, nvDef, nvId, proofDef, proofId, proofPrograms, testId, treeOf } from "./atoms.ts";
import { buildScripts, generatedDigest, generatedIncludes, generatedMeta } from "./build.ts";
import { commitOf } from "./change.ts";
import { CovMap, type Extracted, extract, ItemIndex, type Recorded, recordedIn } from "./extract.ts";
import { readsKeys, testsKey } from "./keys.ts";
import { graphScope, rustFiles } from "./select.ts";
import { type AtomKind, type Keyed, type SelectStore, type Verdict } from "./store.ts";

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

const REC = ".agent-tmp/select-rec";

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

/**
 * Makes `dir`, writable by this account, the administrators and the system alone. `nvs` refuses to
 * write into a directory an ordinary group may write to, and a directory under this repository inherits
 * whatever its drive grants, so a test that makes its scratch under the temporary directory would fail
 * for the place rather than for what it tests.
 */
export async function privateDir(dir: string): Promise<void> {
  mkdirSync(dir, { recursive: true });
  if (process.platform !== "win32") return;
  const user = process.env.USERNAME ?? "";
  const r = await run(["icacls", dir, "/inheritance:r", "/grant:r", "*S-1-5-18:(OI)(CI)F", "*S-1-5-32-544:(OI)(CI)F", `${user}:(OI)(CI)F`], { timeoutMs: 60_000 });
  if (r.code !== 0) throw new Error(`icacls ${dir} failed:\n${r.stdout}${r.stderr}`);
}

/** Runs `jobs` at a time. */
async function pool<T>(items: T[], width: number, body: (item: T) => Promise<void>): Promise<void> {
  let next = 0;
  const worker = async () => {
    while (next < items.length) await body(items[next++]!);
  };
  await Promise.all(Array.from({ length: Math.max(1, Math.min(width, items.length)) }, worker));
}

function chunks<T>(items: T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) out.push(items.slice(i, i + size));
  return out;
}

export async function seed(store: SelectStore, opts: SeedOptions = {}): Promise<SeedReport> {
  const root = opts.root ?? ROOT;
  const say = opts.say ?? ((line: string) => console.log(line));
  const kinds = opts.kinds ?? ["case", "proof", "test", "nv"];
  const jobs = opts.jobs ?? Math.max(2, Math.floor(cpus().length / 2));
  const started = performance.now();
  const rec = join(root, REC);
  rmSync(rec, { recursive: true, force: true });
  await privateDir(join(rec, "tmp"));
  const tmpEnv = { TMP: join(rec, "tmp"), TEMP: join(rec, "tmp"), TMPDIR: join(rec, "tmp") };

  say("select: building covws");
  const build = await buildCovws({ tests: kinds.includes("test") });
  const base = await commitOf("HEAD", root);

  say("select: scanning every Rust file");
  const scanned = scanItems(await rustFiles(root), root);
  store.replaceItems(scanned);
  const index = new ItemIndex(scanned);
  const graph = await metadata();
  const scope = graphScope(graph);
  const generated = generatedIncludes(scanned, (f) => scope.pkgOf(f), root);
  const pkgDirs = new Map([...(graph?.values() ?? [])].map((p) => [p.name, p.dir]));
  const scripts = buildScripts(join(build.dir, "build"), pkgDirs);
  for (const g of generated) store.setMeta(generatedMeta(store.platform, g), generatedDigest(scripts.get(g.pkg), g.name));
  store.setMeta(`build:${store.platform}`, build.nvs);

  const covmap = new CovMap();
  const merge = join(rec, "_merge");
  mkdirSync(merge, { recursive: true });
  const report: SeedReport = { base, kinds: {}, seconds: 0 };
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
  const ctx = { store, build, covmap, index, generated, merge, rec, tmpEnv, jobs, say, tally, root };

  if (kinds.includes("case")) await seedCases(ctx, limit(caseFiles(root), caseId));
  if (kinds.includes("proof")) await seedProofs(ctx, limit(await proofPrograms(build.nvs), (p) => proofId(p.path)));
  if (kinds.includes("test")) await seedTests(ctx, limit([...build.tests].flatMap(([pkg, ts]) => ts.map((t) => ({ pkg, t }))), (x) => testId(x.pkg, x.t)));
  if (kinds.includes("nv")) await seedNv(ctx, limit(nvChecks(root), (c) => nvId(c.id)));

  store.setBase(base);
  rmSync(rec, { recursive: true, force: true });
  report.seconds = Math.round((performance.now() - started) / 1000);
  return report;
}

interface Ctx {
  store: SelectStore;
  build: CovwsBuild;
  covmap: CovMap;
  index: ItemIndex;
  generated: ReturnType<typeof generatedIncludes>;
  merge: string;
  rec: string;
  tmpEnv: Record<string, string>;
  jobs: number;
  say: (line: string) => void;
  tally: (kind: AtomKind, verdict: Verdict, ext: Extracted | null) => void;
  root: string;
}

/** `extract`, with a failure to read an atom's record kept to that atom: its footprint becomes `*`,
 * which every change selects, and the seed goes on. */
async function extractOr(ctx: Ctx, rec: Recorded, objects: string[]): Promise<Extracted> {
  try {
    return await extract(rec, objects, ctx.covmap, ctx.index, ctx.generated, ctx.merge);
  } catch (e) {
    ctx.say(`select: ${rec.name}: its record could not be read, so it depends on everything: ${(e as Error).message.split("\n")[0]}`);
    for (const f of rec.profraws) rmSync(f, { force: true });
    if (rec.log) rmSync(rec.log, { force: true });
    return { keys: new Map([["*", ""]]), executed: 0, unmapped: 0, processes: rec.profraws.length };
  }
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

async function seedCases(ctx: Ctx, files: string[]): Promise<void> {
  const byTree = new Map<string, string[]>();
  for (const f of files) (byTree.get(treeOf(f)) ?? byTree.set(treeOf(f), []).get(treeOf(f))!).push(f);
  let done = 0;
  for (const [tree, cases] of byTree) {
    for (const batch of chunks(cases, 64)) {
      const dir = join(ctx.rec, "cases");
      rmSync(dir, { recursive: true, force: true });
      mkdirSync(dir, { recursive: true });
      const list = join(ctx.rec, "cases.txt");
      writeFileSync(list, batch.join("\n") + "\n");
      const r = await run([ctx.build.nvs, "test", "--cases", list, "--record", dir, "--jobs", String(ctx.jobs), `${tree}/`], {
        cwd: ctx.root,
        env: { ...ctx.tmpEnv, NO_COLOR: "1", NOVIS_NO_FILE_CACHE: "1", LLVM_PROFILE_FILE: join(ctx.rec, "parent", "p-%p.profraw"), NVS_FOOTPRINT_LOG: join(ctx.rec, "parent", "p.log") },
        timeoutMs: 3_600_000,
      });
      const failed = failedLabels(r.stdout);
      const parent = recordedIn(join(ctx.rec, "parent")).get("p");
      const parentKeys: Keyed = parent ? (await extractOr(ctx, parent, [ctx.build.nvs])).keys : new Map();
      // The parent walks the whole tree to find the listed cases, and what else the tree holds decides
      // nothing about one case's verdict: its listings and path tests are not the case's.
      for (const k of [...parentKeys.keys()]) if (/^(dir|exists|tree):/.test(k)) parentKeys.delete(k);
      const recorded = recordedIn(dir);
      await pool(batch, ctx.jobs, async (path) => {
        const got = recorded.get(recordName(path));
        const ext = got ? await extractOr(ctx, got, [ctx.build.nvs]) : null;
        const keys: Keyed = new Map(parentKeys);
        for (const [k, d] of ext?.keys ?? []) keys.set(k, d);
        // A run that crashed before it wrote anything leaves no record; it is red whatever the report says.
        const verdict: Verdict = failed.has(path) || !got || r.timedOut ? "red" : "green";
        ctx.store.recordRun(caseId(path), { def: caseDef(path, ctx.root), verdict, keys });
        ctx.tally("case", verdict, ext);
      });
      done += batch.length;
      ctx.say(`select: cases ${done}/${files.length}`);
    }
  }
}

async function seedProofs(ctx: Ctx, programs: { what: "examples" | "hostile"; path: string }[]): Promise<void> {
  const dir = join(ctx.rec, "proofs");
  const was = process.env[RECORD_ENV];
  const tmp = { TMP: process.env.TMP, TEMP: process.env.TEMP, TMPDIR: process.env.TMPDIR };
  process.env[RECORD_ENV] = dir;
  Object.assign(process.env, ctx.tmpEnv);
  let done = 0;
  try {
    for (const batch of chunks(programs, 96)) {
      rmSync(dir, { recursive: true, force: true });
      mkdirSync(dir, { recursive: true });
      const pass = await runPrograms(namedBinary(ctx.build.nvs), batch, { valgrind: false, cache: false, strict: false });
      const recorded = recordedIn(dir);
      await pool(batch, ctx.jobs, async ({ what, path }) => {
        const got = recorded.get(recordName(path));
        const ext = got ? await extractOr(ctx, got, [ctx.build.nvs]) : null;
        const result = pass.results.get(`${what}:${path}`);
        const verdict: Verdict = !result || result.verdict === "fail" ? "red" : "green";
        ctx.store.recordRun(proofId(path), { def: proofDef(path, ctx.root), verdict, keys: ext?.keys ?? new Map() });
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

async function seedTests(ctx: Ctx, exes: { pkg: string; t: import("../driver/accept.ts").TestExe }[]): Promise<void> {
  const width = Math.max(1, Math.min(4, Math.floor(ctx.jobs / 2)));
  const threads = String(Math.max(1, Math.floor(cpus().length / width)));
  let done = 0;
  await pool(exes, width, async ({ pkg, t }) => {
    const id = testId(pkg, t);
    const name = recordName(id);
    const dir = join(ctx.rec, "tests", name);
    mkdirSync(dir, { recursive: true });
    // The binary's own compile cache starts empty, so the first compile of each program runs whole.
    // `NOVIS_NO_FILE_CACHE` is not set: the binaries that test the cache would test nothing.
    const cache = join(ctx.rec, "cache", name);
    await privateDir(cache);
    const r = await run([t.exe], {
      cwd: t.dir,
      env: {
        ...ctx.tmpEnv,
        CARGO_MANIFEST_DIR: t.dir,
        RUST_TEST_THREADS: threads,
        NO_COLOR: "1",
        LOCALAPPDATA: cache,
        XDG_CACHE_HOME: cache,
        // A merge pool: every process of one binary adds its counters to one of a few files, since a test
        // binary can start hundreds of `nvs` processes and each would leave a profile of its own.
        LLVM_PROFILE_FILE: join(dir, `${name}-%4m.profraw`),
        NVS_FOOTPRINT_LOG: join(dir, `${name}.log`),
      },
      timeoutMs: 3_600_000,
    });
    const got = recordedIn(dir).get(name);
    const ext = got ? await extractOr(ctx, got, [t.exe, ctx.build.nvs]) : null;
    const verdict: Verdict = r.code === 0 && got ? "green" : "red";
    // A test added to its package cannot be in any footprint yet, so every binary of the package holds
    // this key, which an added test moves; a binary that ran no instrumented code holds only this one.
    const keys: Keyed = new Map(ext?.keys ?? []);
    keys.set(testsKey(pkg), "");
    ctx.store.recordRun(id, { def: "", verdict, keys });
    ctx.tally("test", verdict, ext);
    rmSync(dir, { recursive: true, force: true });
    rmSync(cache, { recursive: true, force: true });
    ctx.say(`select: test binaries ${++done}/${exes.length}${verdict === "red" ? ` (${id} red)` : ""}`);
  });
}

/** The `bun nv` command checks of the live plan a seed runs. */
export function nvChecks(root: string = ROOT): { id: string; argv: string[]; cwd: string }[] {
  let live: string | null = null;
  try {
    live = (JSON.parse(readFileSync(join(root, "data", "chain.json"), "utf8")) as { live: string | null }).live;
  } catch {
    return [];
  }
  if (!live) return [];
  const checks = (goalPlan(live, root)?.checks ?? []) as { id: string; kind: string; argv?: string[]; cwd?: string }[];
  return checks
    .filter((c) => c.kind === "command" && (c.argv ?? []).slice(0, 2).join(" ") === "bun nv" && seedable(c.argv!))
    .map((c) => ({ id: c.id, argv: c.argv!, cwd: c.cwd ?? "." }));
}

async function seedNv(ctx: Ctx, checks: { id: string; argv: string[]; cwd: string }[]): Promise<void> {
  const dir = join(ctx.rec, "nv");
  mkdirSync(dir, { recursive: true });
  let done = 0;
  await pool(checks, Math.min(4, ctx.jobs), async (c) => {
    const log = join(dir, `${recordName(c.id)}.reads`);
    const r = await run(c.argv, { cwd: join(ctx.root, c.cwd), env: { ...ctx.tmpEnv, NO_COLOR: "1", NV_READS_LOG: log }, timeoutMs: 900_000 });
    const reads = existsSync(log) ? readLog(log) : null;
    const keys: Keyed = new Map();
    // A check that left no record of its reads read anything it liked: it is under every change.
    if (!reads) keys.set("*", "");
    else for (const k of readsKeys(reads, readModules(log))) keys.set(k, "");
    rmSync(log, { force: true });
    const verdict: Verdict = r.code === 0 ? "green" : "red";
    ctx.store.recordRun(nvId(c.id), { def: nvDef(c.argv, c.cwd), verdict, keys });
    ctx.tally("nv", verdict, null);
    ctx.say(`select: bun nv checks ${++done}/${checks.length}`);
  });
}
