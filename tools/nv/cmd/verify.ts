// `bun nv verify`: AGENTS.md § *Session workflow* step 3, as one command.
//
//     bun nv verify                  every step
//     bun nv verify -p nvs-ir        the same build; only nvs-ir's test binaries run
//     bun nv verify --fast           build and test only, for a mid-work check
//     bun nv verify --doc            the rustdoc gate alone; the driver's goal-end call
//     bun nv verify --start          run it detached through `nv bg` and return at once
//     bun nv verify --wait           collect what --start left, with its exit status
//     bun nv verify --full           do not truncate the failing step's output
//     bun nv verify --no-cache       run every step, whatever the green cache holds
//     bun nv verify --list           the steps in order, running none of them
//     bun nv verify --keys           each test binary's key over the tree now, as JSON
//
// The steps, in order, stopping at the first failure: `cargo fmt`, `bun nv lints --check`,
// `bun nv directives --check` and `--check-template`, `bun nv owners --check`, `bun nv selftest`,
// the fuzz workspace's lock brought back in step, `cargo build`, `nvs fmt` over the `.nvs` files this
// working tree added or changed, `cargo test`, the `.nvst` trees through the debug binary the build
// produced, `bun nv reference`, `cargo clippy --all-targets -- -D warnings`, and the VS Code
// extension's headless suites. Green prints one line per step; a failure prints that step's output and
// nothing else. The full output of every step is written to `.agent-tmp/verify-<step>.log`. This
// command judges nothing: a step's own exit status is the whole verdict.
//
// `fmt` and `nvs-fmt` format rather than check, because a red `fmt` was only ever fixed by running the
// formatter and verifying again. `fmt` runs first, so everything after it compiles the text the commit
// carries, and its own failure is reported only when every other step passed: a parse error reads
// better from `build`. `nvs-fmt` runs over only the new or modified `.nvs` files under `tests/` and
// `examples/`, `tests/fmt/input/` excepted, so a layout rule that changes what the formatter prints is
// still `crates/nvs-fmt/tests/identity.rs`'s to judge over the rest. A file the formatter cannot parse
// is left as it was, and the step is never red. After either one rewrites a file, every later key is
// taken again.
//
// A step whose key has not moved since it was last green is answered from `.agent-tmp/verify-green.json`
// rather than run. `tools/nv/keys/steps.ts` is what each step reads. Only a green step is recorded, the
// moment it passes; an entry expires after an hour; a red step's entry is deleted. `build` is answered
// from the cache only when every later step that uses what it leaves on disk is too, or, for `test`,
// when the test binaries cargo last built are provably the ones this code compiles to: the build key
// they were built under still holds, and every file cargo produced then is untouched. A `-p` run's
// `test` verdict is keyed with its package, and an unscoped verdict satisfies any `-p`.
//
// `test` runs the binaries `cargo test --no-run` names side by side, as many at a time as there are
// cores, the slowest of the last run first, each with libtest threads in proportion to its last time,
// and `cargo test --doc` beside them. Every binary runs, and one that fails is run a second time, alone:
// a binary that passes alone shares a port, a path or a container with another, and is reported red
// with that diagnosis rather than retried into green. A binary whose key in `tools/nv/keys/checks.ts`
// has not moved is answered from `.agent-tmp/verify-test-green.json`, with the `test result:` line and
// the test lines it printed when green, which the loop driver reads too. `--keys` prints each binary the
// last build recorded, with its key over the tree now and whether that key is wide, so the driver
// compares a record with the same key this command would. Each binary run records the
// paths it opened through `nvs_repo` in the file `NVS_READS_LOG` names, and those go into its key.
// An unscoped run fails on a wide binary `tools/data/impact-wide.txt` does not list, and on a listed
// one that is narrow now (`tools/nv/keys/escape.ts`).
//
// Two stretches run at the same time and are still judged in list order. The script steps read nothing
// `build` writes, so they run beside it. Once every test binary has started and at most half the cores
// are busy, the steps after `test` start on one lane, one at a time; nothing starts there after a binary
// has failed, and the lane stops before a failed binary is run again alone. A red `test` drops the
// lane's verdicts.
//
// `cargo doc` with broken intra-doc links denied is `--doc`, run alone. It re-documents every crate
// above an edit, so the loop driver runs it once, when a goal's acceptance list is green. The
// documentation gates (`rules`, `records`, links, the plan, the playbook) are not steps: the green cache
// deliberately does not key on prose, so a gate behind it would be skipped exactly when prose changed.
// `nv session --wrap` refuses a wrap that breaks them.
//
// `.agent-tmp/verify-progress.json` names the step in flight, for the loop driver's status line.
//
// Exits 0 when every step is green, 1 when one is red, and 2 on a bad argument or a `--wait` with nothing
// to collect.

import { cpus } from "node:os";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";
import { ArgError, parseArgs } from "../lib/py.ts";
import { type Unit, isWide, loadRecords, units } from "../keys/checks.ts";
import { findings } from "../keys/escape.ts";
import { type Graph, metadata } from "../keys/graph.ts";
import { keyOf } from "../keys/key.ts";
import { digest } from "../keys/scan.ts";
import { STEP_READS, stepKey } from "../keys/steps.ts";
import { Tree } from "../keys/tree.ts";
import { alive, dirOf, exitOf, readJob, startJob } from "./bg.ts";

export const summary = "the gate, one call: nv verify [-p <crate>] [--fast] [--doc] [--start | --wait] [--list] [--full] [--no-cache]";

const TMP = join(ROOT, ".agent-tmp");
const CACHE = join(TMP, "verify-green.json");
/** The green cache's shape. A file in any other shape reads as empty. */
const CACHE_SHAPE = 3;
const PROGRESS = join(TMP, "verify-progress.json");
/** Each test binary's seconds in the last run, so the next one starts the slowest first. */
const TEST_TIMES = join(TMP, "verify-test-times.json");
/** What the last `cargo test --no-run` built, and under which build key. */
const TEST_BUILT = join(TMP, "verify-test-built.json");
/** Each test binary's green verdict: `{name: {key, result, tests}}`. */
const TEST_GREEN = join(TMP, "verify-test-green.json");
/** Each test binary's recorded run-time reads, which `checks.ts` keys it on. */
const READS = join(TMP, "impact-reads.json");
const READS_DIR = join(TMP, "reads");
const READS_ENV = "NVS_READS_LOG";
/** The job id of the last `--start`. */
const BACKGROUND = join(TMP, "verify-background.json");
const BACKGROUND_TIMEOUT_S = 600;

const TAIL_LINES = 60;
const CACHE_TTL_S = 3600;
/** A step is killed past this, so a hung test cannot hold a session forever. */
const STEP_TIMEOUT_MS = 60 * 60 * 1000;

/** The steps that use what `build` leaves on disk. */
const NEEDS_BINARY = new Set(["nvs-fmt", "test", "conformance", "differential", "reference", "extension"]);
/** The steps that rewrite source, after which every later key is taken again. */
const WRITES = new Set(["fmt", "nvs-fmt"]);
/** The script steps, which run at the same time as `build`. */
const BESIDE_BUILD = new Set(["lints", "directives", "template", "owners", "nv", "fuzz-lock"]);
const NVS_FMT_TREES = ["tests", "examples"];
const NVS_FMT_SKIPS = "tests/fmt/input/";
const EXTENSION = join(ROOT, "editors", "vscode");
const FUZZ = join(ROOT, "fuzz");
/** The `.nvst` trees, run through the debug binary, which is the one the loop's acceptance check runs. */
const CASE_TREES = ["conformance", "differential"];
const NVS = join(ROOT, "target", "debug", process.platform === "win32" ? "nvs.exe" : "nvs");

const RESULT_RE = /test result: \w+\. (\d+) passed; (\d+) failed/g;
/** One test's line in libtest's output, a `#[should_panic]` test's included. */
const TEST_LINE_RE = /^test \S+ (?:- should panic )?\.\.\. \w+/;
const CASES_RE = /(\d+) passed, (\d+) failed, (\d+) skipped/;
const WARN_RE = /^(warning|error)(\[[^\]]+\])?: (.*)$/gm;

interface Opts {
  package?: string;
  fast: boolean;
  doc: boolean;
  full: boolean;
  noCache: boolean;
  start: boolean;
  wait: boolean;
  list: boolean;
}

interface Job {
  name: string;
  owner: string;
  argv: string[];
  cwd: string;
  env: Record<string, string>;
  rerun: string;
  readsLog?: string;
}

/** One reading of the tree, and the graph its build keys are taken over. */
interface Ctx {
  tree: Tree;
  graph: Graph | null;
  keys: Map<string, string | null>;
}

interface Step {
  name: string;
  args: string[];
  summarize: (out: string) => string;
  exe: string;
  cwd: string;
  env?: Record<string, string>;
  /** Replaces the one command; `args` is then what a reader types to reproduce the step. */
  runner?: (s: Step) => Promise<[number, string]>;
  seconds: number;
  code: number | null;
  out: string;
  // `nvs-fmt`'s state: what it still has to format, every changed file, and each digest it left.
  todo?: string[];
  changed?: string[];
  formatted?: Record<string, string>;
  // `test`'s state.
  binaryKey?: string | null;
  skipDoc?: boolean;
  docGreen?: boolean;
  noCache?: boolean;
  ctx?: Ctx | null;
  onTail?: () => void;
  quiesce?: () => Promise<void>;
}

interface Entry {
  key?: string;
  when?: number;
  summary?: string;
  seconds?: number;
}

interface Cache {
  shape: number;
  steps: Record<string, Entry>;
  formatted: Record<string, string>;
}

function step(name: string, args: string[], summarize: (out: string) => string, extra: Partial<Step> = {}): Step {
  return { name, args, summarize, exe: "cargo", cwd: ROOT, seconds: 0, code: null, out: "", ...extra };
}

const cmdOf = (s: Step) => `${s.exe} ${s.args.join(" ")}`;
const now = () => Date.now() / 1000;

function readJson(path: string): unknown {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return undefined;
  }
}

function readObject<T>(path: string): Record<string, T> {
  const got = readJson(path);
  return got && typeof got === "object" && !Array.isArray(got) ? (got as Record<string, T>) : {};
}

function sorted<T>(obj: Record<string, T>): Record<string, T> {
  return Object.fromEntries(Object.keys(obj).sort().map((k) => [k, obj[k]!]));
}

/** Best effort: a file under `.agent-tmp` that cannot be written costs the next run a cache miss. */
function writeQuiet(path: string, text: string): void {
  try {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, text);
  } catch {
    // See above.
  }
}

// ---- running ---------------------------------------------------------------------------------------

async function spawnOut(argv: string[], cwd: string, env?: Record<string, string>): Promise<[number, string]> {
  try {
    const p = await proc(argv, { cwd, timeoutMs: STEP_TIMEOUT_MS, ...(env ? { env } : {}) });
    return [p.code, p.stdout + p.stderr + (p.timedOut ? `\nkilled after ${STEP_TIMEOUT_MS / 60000} minutes\n` : "")];
  } catch (e) {
    return [-1, `could not run \`${argv.join(" ")}\`: ${(e as Error).message}`];
  }
}

async function runStep(s: Step): Promise<boolean> {
  const started = performance.now();
  [s.code, s.out] = s.runner ? await s.runner(s) : await spawnOut([s.exe, ...s.args], s.cwd, s.env);
  s.seconds = (performance.now() - started) / 1000;
  writeQuiet(join(TMP, `verify-${s.name}.log`), s.out);
  return s.code === 0;
}

// ---- the test step ---------------------------------------------------------------------------------

/** The jobs the last build recorded, if they are still what this code compiles to: the same build key,
 * and every file cargo produced then untouched since. `null` sends `test` back to cargo. */
function jobsOnDisk(binaryKey: string | null | undefined): Job[] | null {
  if (!binaryKey) return null;
  const built = readJson(TEST_BUILT) as { binary?: string; jobs?: Job[]; files?: Record<string, [string, number]> } | undefined;
  if (!built || built.binary !== binaryKey || !built.jobs?.length || !built.files) return null;
  try {
    for (const [path, [mtime, size]] of Object.entries(built.files)) {
      const st = statSync(path, { bigint: true });
      if (st.mtimeNs.toString() !== mtime || Number(st.size) !== size) return null;
    }
  } catch {
    return null;
  }
  return built.jobs;
}

const FLAGS: Record<string, string> = { lib: "--lib", bin: "--bin", test: "--test", example: "--example", bench: "--bench" };

/** `cargo test --no-run`, as every workspace test binary's job, or `[null, output]` when it fails. A
 * `-p` never goes on the build: it narrows which binaries run, off the one build (AGENTS.md rule 5). */
async function buildTestJobs(binaryKey: string | null | undefined): Promise<[Job[] | null, string]> {
  let p;
  try {
    p = await proc(["cargo", "test", "--no-run", "--message-format=json-render-diagnostics"], { timeoutMs: STEP_TIMEOUT_MS });
  } catch (e) {
    return [null, `could not run \`cargo test --no-run\`: ${(e as Error).message}`];
  }
  if (p.code !== 0) return [null, p.stderr + p.stdout];
  const jobs: Job[] = [];
  const files: Record<string, [string, number]> = {};
  for (const line of p.stdout.split("\n")) {
    let m: {
      reason?: string;
      package_id?: string;
      filenames?: string[];
      executable?: string | null;
      profile?: { test?: boolean };
      target: { name: string; kind?: string[] };
      manifest_path: string;
    };
    try {
      m = JSON.parse(line);
    } catch {
      continue;
    }
    if (m.reason !== "compiler-artifact") continue;
    const id = m.package_id ?? "";
    if (id.startsWith("path+")) {
      // Every file of a workspace package, not the test binaries alone: a test spawns
      // `target/debug/nvs`, and that is the `bin` artifact beside it.
      for (const name of m.filenames ?? []) {
        try {
          const st = statSync(name, { bigint: true });
          files[name] = [st.mtimeNs.toString(), Number(st.size)];
        } catch {
          // A file that is gone is simply not recorded.
        }
      }
    }
    if (!m.executable || !m.profile?.test) continue;
    const hash = id.lastIndexOf("#");
    const source = hash < 0 ? "" : id.slice(0, hash);
    const tail = hash < 0 ? id : id.slice(hash + 1);
    const owner = tail.includes("@") ? tail.split("@")[0]! : source.replace(/\/+$/, "").split("/").pop()!;
    const target = m.target.name;
    const kind = (m.target.kind ?? []).find((k) => k in FLAGS) ?? "lib";
    const cwd = dirname(m.manifest_path);
    // The reproduction stays inside the same build: a target flag narrows the run, a `-p` would not.
    const rerun = kind === "lib" ? `bun nv verify -p ${owner}` : `cargo test ${FLAGS[kind]} ${target}`;
    jobs.push({ name: `${owner} ${kind} ${target}`, owner, argv: [m.executable], cwd, env: { CARGO_MANIFEST_DIR: cwd }, rerun });
  }
  if (binaryKey && jobs.length > 0) writeQuiet(TEST_BUILT, JSON.stringify({ binary: binaryKey, jobs, files }));
  return [jobs, ""];
}

/** What `cargo test` would run, as jobs, or `[null, output]` when the build fails. A scoped run skips
 * the doc-tests, which cargo can only narrow with a `-p`. */
async function testJobs(pkg: string | undefined, binaryKey: string | null | undefined): Promise<[Job[] | null, string]> {
  let jobs = jobsOnDisk(binaryKey);
  let note = "test binaries as last built: this code and those files are unchanged\n";
  if (jobs === null) {
    [jobs, note] = await buildTestJobs(binaryKey);
    if (jobs === null) return [null, note];
  }
  jobs = jobs.filter((j) => !pkg || j.owner === pkg);
  if (!pkg) jobs.push({ name: "doc-tests", owner: "", argv: ["cargo", "test", "--doc"], cwd: ROOT, env: {}, rerun: "cargo test --doc" });
  return [jobs, note];
}

async function runJob(job: Job): Promise<[number, number, string]> {
  const started = performance.now();
  const [code, out] = await spawnOut(job.argv, job.cwd, job.env);
  return [(performance.now() - started) / 1000, code, out];
}

interface Green {
  key?: string;
  result?: string;
  tests?: string[];
}

/** Each test binary's key over what it reads, and the run-time reads a run records into it. */
class Reach {
  private readonly records;
  private readonly units: Map<string, Unit>;
  private readonly keys = new Map<string, string>();
  private readonly recorded: Record<string, { reads: string[] }>;

  constructor(
    private readonly ctx: Ctx,
    readonly graph: Graph,
  ) {
    this.records = loadRecords(graph, false);
    this.units = new Map(units(this.records).filter((u) => u.role === "binary").map((u) => [u.name, u]));
    this.recorded = readObject(READS);
  }

  /** A binary with no unit keys on everything `test` reads. */
  key(job: Job): string {
    let k = this.keys.get(job.name);
    if (k === undefined) {
      const u = this.units.get(job.name);
      k = keyOf(job.name, u ? u.parts(this.ctx.tree) : STEP_READS.test!(this.ctx.tree, this.graph));
      this.keys.set(job.name, k);
    }
    return k;
  }

  /** Is `job`'s key on everything? A binary with no unit is. */
  wide(job: Job): boolean {
    const u = this.units.get(job.name);
    return !u || isWide(u.parts(this.ctx.tree));
  }

  reads(name: string): string[] | undefined {
    return this.records.reads.get(name);
  }

  /** Files what a run of `job` appended to its reads log. */
  record(job: Job): void {
    let lines: string[] = [];
    try {
      lines = readFileSync(job.readsLog!, "utf8").split(/\r?\n/);
    } catch {
      // A binary that opened nothing through `nvs_repo` writes no log.
    }
    const reads = [...new Set(lines.map((l) => l.trim().replace(/\\/g, "/").replace(/^\/+|\/+$/g, "")).filter((l) => l))].sort();
    this.records.reads.set(job.name, reads);
    this.recorded[job.name] = { reads };
    this.keys.delete(job.name);
  }

  save(): void {
    writeQuiet(READS, JSON.stringify(sorted(this.recorded), null, 1));
  }
}

const RERUN_ALONE_PASSED = (name: string) =>
  `error: \`${name}\` failed beside the other test binaries and passed alone. It shares something with a ` +
  `binary that runs at the same time -- a fixed port, a fixed path under the shared temp or target directory, ` +
  `a container name -- or leans on a timeout that load breaks. Give the test its own (port 0, a directory no ` +
  `other test names, its own container) rather than running it apart: \`tools/nv/cmd/verify.ts\`'s module doc.`;

/** The `test` step. `pkg` is `-p`: which binaries run, off the one build. */
async function runTests(s: Step, pkg: string | undefined): Promise<[number, string]> {
  const [all, note] = await testJobs(pkg, s.binaryKey);
  if (all === null) return [1, note];
  let jobs = s.skipDoc ? all.filter((j) => j.name !== "doc-tests") : all;
  const last = readObject<number>(TEST_TIMES);

  // Which binaries this change reaches. With no tree or graph to key against, every binary runs and
  // nothing is remembered.
  const reach = s.ctx?.graph ? new Reach(s.ctx, s.ctx.graph) : null;
  const green = reach && !s.noCache ? readObject<Green>(TEST_GREEN) : {};
  const binaries = jobs.filter((j) => j.name !== "doc-tests");
  const held: Record<string, Green> = {};
  if (reach) for (const j of binaries) if (green[j.name]?.key === reach.key(j)) held[j.name] = green[j.name]!;
  jobs = jobs.filter((j) => !(j.name in held));
  for (const j of jobs) {
    if (!reach || j.name === "doc-tests") continue;
    const log = join(READS_DIR, j.name.replace(/\W+/g, "-") + ".log");
    try {
      mkdirSync(READS_DIR, { recursive: true });
      rmSync(log, { force: true });
    } catch {
      // A log that cannot be cleared only over-records.
    }
    j.env = { ...j.env, [READS_ENV]: log };
    j.readsLog = log;
  }

  // Unknown first: a binary with no recorded time is new, and new is as likely to be slow.
  const time = (j: Job) => (typeof last[j.name] === "number" ? last[j.name]! : Infinity);
  jobs.sort((a, b) => (time(b) > time(a) ? 1 : time(b) < time(a) ? -1 : 0));
  const workers = cpus().length || 4;
  // libtest's threads, per binary, in proportion to its last time against the slowest one's.
  const known = jobs.map(time).filter((t) => t !== Infinity);
  const slowest = known.length ? Math.max(...known) : 0;
  for (const j of jobs) {
    const share = slowest > 0 && time(j) !== Infinity ? time(j) / slowest : 1;
    j.env = { ...j.env, RUST_TEST_THREADS: String(Math.max(1, Math.min(workers, Math.round(workers * share)))) };
  }

  // The tail: every job started and at most half the cores still running one.
  const results = new Map<string, [number, number, string]>();
  let left = jobs.length;
  let red = false;
  let next = 0;
  const worker = async () => {
    while (next < jobs.length) {
      const j = jobs[next++]!;
      const got = await runJob(j);
      results.set(j.name, got);
      left--;
      red ||= got[1] !== 0;
      if (s.onTail && !red && left <= Math.floor(workers / 2)) s.onTail();
    }
  };
  await Promise.all(Array.from({ length: Math.min(workers, jobs.length) }, worker));

  const failed = jobs.filter((j) => results.get(j.name)![1] !== 0);
  s.docGreen = results.has("doc-tests") && results.get("doc-tests")![1] === 0;
  // Alone, one at a time, after the pool has drained and the lane has stopped.
  if (failed.length && s.quiesce) await s.quiesce();
  const alone = new Map<string, boolean>();
  for (const j of failed) alone.set(j.name, (await runJob(j))[1] === 0);

  let wide: string[] = [];
  if (reach) {
    const stored = readObject<Green>(TEST_GREEN);
    for (const j of jobs) {
      if (j.name === "doc-tests") continue;
      const [, code, text] = results.get(j.name)!;
      if (code !== 0) {
        delete stored[j.name];
        continue;
      }
      // What it opened first, because the key that stands for this run holds those reads.
      reach.record(j);
      const out = text.split(/\r?\n/);
      stored[j.name] = {
        key: reach.key(j),
        result: out.filter((l) => new RegExp(RESULT_RE.source).test(l)).join("\n"),
        tests: out.filter((l) => TEST_LINE_RE.test(l)).map((l) => l.trimEnd()),
      };
    }
    reach.save();
    writeQuiet(TEST_GREEN, JSON.stringify(sorted(stored), null, 1));
    // Unscoped runs only: the wide list is the workspace's, and `-p` sees one package's.
    if (!pkg) {
      wide = findings(binaries.map((j) => ({ name: j.name, owner: j.owner, exe: j.argv[0]!, reads: reach.reads(j.name), known: reach.graph.has(j.owner) })));
    }
  }

  const times: Record<string, number> = { ...last };
  for (const [n, r] of results) times[n] = Math.round(r[0] * 100) / 100;
  writeQuiet(TEST_TIMES, JSON.stringify(sorted(times), null, 1));

  // Passing binaries first, by name, so the tail a red step prints is the failures.
  const names = new Set(failed.map((j) => j.name));
  const out: string[] = note ? [note] : [];
  const heldNames = Object.keys(held).sort();
  if (heldNames.length) out.push(`${heldNames.length} of ${binaries.length} test binaries not re-run: nothing each one reads has changed since it was green`);
  for (const n of heldNames) out.push(`     Unchanged ${n}\n${held[n]!.result ?? ""}`);
  for (const n of [...results.keys()].sort()) if (!names.has(n)) out.push(`     Running ${n}\n${results.get(n)![2]}`);
  for (const j of failed) out.push(`     Running ${j.name}  -- FAILED, exit ${results.get(j.name)![1]}\n${results.get(j.name)![2]}`);
  for (const j of failed) out.push(alone.get(j.name) ? RERUN_ALONE_PASSED(j.name) : `error: \`${j.name}\` failed, alone as well; \`${j.rerun}\` runs it again.`);
  for (const line of wide) out.push(`error: ${line}`);
  return [failed.length || wide.length ? 1 : 0, out.join("\n")];
}

// ---- nvs-fmt ---------------------------------------------------------------------------------------

/** The `.nvs` files git reports as new or modified under `NVS_FMT_TREES`, repo-relative. */
async function changedSources(): Promise<string[]> {
  let p;
  try {
    p = await proc(["git", "status", "--porcelain", "-z", "--untracked-files=all", "--", ...NVS_FMT_TREES]);
  } catch {
    return [];
  }
  if (p.code !== 0) return [];
  const found: string[] = [];
  const entries = p.stdout.split("\0");
  for (let i = 0; i < entries.length; ) {
    const entry = entries[i++]!;
    if (entry.length < 4) continue;
    const status = entry.slice(0, 2);
    const rel = entry.slice(3);
    // `-z` puts the name a rename or copy came from in the next field.
    if (status[0] === "R" || status[0] === "C") i++;
    if (status.includes("D") || !rel.endsWith(".nvs") || rel.startsWith(NVS_FMT_SKIPS)) continue;
    try {
      if (!statSync(join(ROOT, rel)).isFile()) continue;
    } catch {
      continue;
    }
    found.push(rel);
  }
  return found.sort();
}

const digestOf = (rel: string) => digest(readFileSync(join(ROOT, rel)));

/** `[todo, changed]`: the changed `.nvs` files the formatter has not been over as they now stand, and
 * all of the changed ones, which are the only paths the cache still has a reason to hold. */
async function unformatted(cache: Cache): Promise<[string[], string[]]> {
  const changed = await changedSources();
  const todo: string[] = [];
  for (const rel of changed) {
    try {
      if (cache.formatted[rel] !== digestOf(rel)) todo.push(rel);
    } catch {
      // A file gone since git listed it has nothing to format.
    }
  }
  return [todo, changed];
}

/** The `nvs-fmt` step. Always exit 0: a refusal is a file that does not parse, and whether it was meant
 * to is a test's verdict. */
async function runNvsFmt(s: Step): Promise<[number, string]> {
  const todo = s.todo ?? (await changedSources());
  s.formatted = {};
  if (todo.length === 0) return [0, ""];
  if (!existsSync(s.exe)) return [0, `note: ${s.exe} is not built, so nothing was formatted`];
  const before = new Map(todo.map((rel) => [rel, readFileSync(join(ROOT, rel))]));
  const [, said] = await spawnOut([s.exe, "fmt", ...todo], ROOT);
  const lines: string[] = [];
  for (const rel of todo) {
    let after: Buffer;
    try {
      after = readFileSync(join(ROOT, rel));
    } catch {
      continue;
    }
    s.formatted[rel] = digest(after);
    if (!after.equals(before.get(rel)!)) lines.push(`rewrote ${rel}`);
  }
  lines.push(`looked at ${todo.length} new or modified file(s)`);
  return [0, lines.join("\n") + "\n\n" + said];
}

// ---- fuzz-lock -------------------------------------------------------------------------------------

/** Resolves the `fuzz/` workspace, which rewrites `fuzz/Cargo.lock` when it has fallen behind. `fuzz/`
 * is its own workspace, so nothing the root build does touches its lock; resolving here puts the rewrite
 * in the session that added the dependency. `cargo metadata` resolves without compiling or upgrading,
 * offline first because the root build has already fetched what a workspace crate newly depends on. */
async function runFuzzLock(): Promise<[number, string]> {
  const lock = join(FUZZ, "Cargo.lock");
  const read = () => (existsSync(lock) ? readFileSync(lock) : Buffer.alloc(0));
  const before = read();
  const args = ["cargo", "metadata", "--format-version", "1", "--manifest-path", join(FUZZ, "Cargo.toml")];
  let said = "";
  let code = 1;
  for (const extra of [["--offline"], []]) {
    try {
      const p = await proc([...args, ...extra], { timeoutMs: STEP_TIMEOUT_MS });
      said += p.stderr;
      code = p.code;
    } catch (e) {
      said += `could not run cargo: ${(e as Error).message}\n`;
      code = -1;
    }
    if (code === 0) break;
  }
  if (code === 0) said += read().equals(before) ? "fuzz-lock: fuzz/Cargo.lock in step\n" : "fuzz-lock: rewrote fuzz/Cargo.lock\n";
  return [code, said];
}

// ---- summaries -------------------------------------------------------------------------------------

const NO_SUMMARY = "ran, but printed no summary line -- check the log";

function named(files: string[]): string {
  const more = files.length > 3 ? `, +${files.length - 3} more` : "";
  return `formatted ${files.length} file(s): ${files.slice(0, 3).map((f) => basename(f)).join(", ")}${more}`;
}

function warnings(out: string): number {
  return [...out.matchAll(WARN_RE)].filter((m) => m[1] === "warning").length;
}

export const summaries: Record<string, (out: string) => string> = {
  build: () => "ok",
  test(out) {
    const suites = [...out.matchAll(RESULT_RE)];
    if (suites.length === 0) return "ran, but printed no `test result:` line -- check the log";
    const passed = suites.reduce((n, m) => n + Number(m[1]), 0);
    const failed = suites.reduce((n, m) => n + Number(m[2]), 0);
    const unchanged = (out.match(/^ {5}Unchanged /gm) ?? []).length;
    return `${passed} passed, ${failed} failed  (${suites.length} suites${unchanged ? `, ${unchanged} binaries not re-run` : ""})`;
  },
  clippy: (out) => (warnings(out) === 0 ? "no warnings" : `${warnings(out)} warning(s)`),
  doc: (out) => (warnings(out) === 0 ? "every link resolves" : `${warnings(out)} warning(s)`),
  fmt(out) {
    const files = out.split("\n").map((l) => l.trim()).filter((l) => l.endsWith(".rs"));
    return files.length === 0 ? "clean" : named(files);
  },
  "nvs-fmt"(out) {
    const files = out.split("\n").filter((l) => l.startsWith("rewrote ")).map((l) => l.slice("rewrote ".length));
    const looked = /^looked at (\d+) /m.exec(out);
    if (files.length === 0) return looked ? `clean (${looked[1]} new or modified)` : "nothing new to format";
    return named(files);
  },
  cases(out) {
    const m = CASES_RE.exec(out);
    if (!m) return "ran, but printed no `N passed` line -- check the log";
    return `${m[1]} passed, ${m[2]} failed` + (Number(m[3]) ? `, ${m[3]} skipped` : "");
  },
  extension(out) {
    const m = /(\d+)\s+passing/.exec(out);
    return m ? `${m[1]} passing` : "ran, but printed no `N passing` line -- check the log";
  },
  reference(out) {
    const m = /(\d+) of (\d+) examples hold/.exec(out);
    return m ? `${m[1]} of ${m[2]} examples hold` : "ran, but printed no `N of M examples hold` line -- check the log";
  },
  lints(out) {
    let m = /lints: (\d+) generated tables current/.exec(out);
    if (m) return `${m[1]} generated lint tables current`;
    m = /(\d+) problem\(s\)/.exec(out);
    return m ? `${m[1]} lint table(s) drifted -- run \`bun nv lints\`` : NO_SUMMARY;
  },
  template(out) {
    let m = /spells all (\d+) leaf keys/.exec(out);
    if (m) return `the default file spells all ${m[1]} leaf keys, each once`;
    m = /(\d+) problem\(s\) over (\d+) leaf key/.exec(out);
    return m ? `${m[1]} of ${m[2]} leaf keys are wrong in the default file` : NO_SUMMARY;
  },
  directives(out) {
    let m = /directives: (\d+) leaf keys/.exec(out);
    if (m) return `${m[1]} leaf keys, every one read or declared`;
    m = /(\d+) problem\(s\) over (\d+) leaf key/.exec(out);
    return m ? `${m[1]} of ${m[2]} leaf keys unread and undeclared` : NO_SUMMARY;
  },
  owners(out) {
    let m = /every one of the (\d+) tagged gap\(s\)/.exec(out);
    if (m) return `${m[1]} recorded gap(s), each deferred to a milestone still ahead`;
    m = /(\d+) recorded gap\(s\) name an owner this gate refuses/.exec(out);
    return m ? `${m[1]} recorded gap(s) name an owner the gate refuses` : NO_SUMMARY;
  },
  nv(out) {
    if (out.includes("nv selftest: every test passes")) {
      const m = /^\s*(\d+) pass$/m.exec(out);
      return m ? `the types check, ${m[1]} test(s) pass` : "the types check, every test passes";
    }
    if (out.includes("nv selftest: the types check")) return "the types check, and `bun test` failed -- check the log";
    return "`tsc` failed or did not run -- check the log";
  },
  "fuzz-lock"(out) {
    if (out.includes("rewrote fuzz/Cargo.lock")) return "fuzz/Cargo.lock had fallen behind the workspace and was rewritten -- commit it";
    return out.includes("in step") ? "fuzz/Cargo.lock in step with the workspace" : NO_SUMMARY;
  },
};

// ---- the steps -------------------------------------------------------------------------------------

/** The rustdoc gate. `private_intra_doc_links` is allowed: a link to a crate-private item in a crate
 * nobody publishes is a correct reference rustdoc will not turn into an anchor. Always the whole
 * workspace, whatever `-p` says. */
function docStep(): Step {
  return step("doc", ["doc", "--no-deps", "--workspace"], summaries.doc!, { env: { RUSTDOCFLAGS: "-A rustdoc::private_intra_doc_links -D warnings" } });
}

function stepsFor(opts: Opts): Step[] {
  if (opts.doc) return [docStep()];
  const steps: Step[] = [];
  if (!opts.fast) {
    // `-l` names each file rustfmt rewrote, which the summary quotes.
    steps.push(step("fmt", ["fmt", "--all", "--", "-l"], summaries.fmt!));
    // The script steps decide what the tree means rather than whether it builds, and read manifests,
    // doc comments or the tools, so `-p` narrows none of them.
    steps.push(step("lints", ["nv", "lints", "--check"], summaries.lints!, { exe: "bun" }));
    steps.push(step("directives", ["nv", "directives", "--check"], summaries.directives!, { exe: "bun" }));
    steps.push(step("template", ["nv", "directives", "--check-template"], summaries.template!, { exe: "bun" }));
    steps.push(step("owners", ["nv", "owners", "--check"], summaries.owners!, { exe: "bun" }));
    if (existsSync(join(ROOT, "package.json"))) steps.push(step("nv", ["nv", "selftest"], summaries.nv!, { exe: "bun" }));
    if (existsSync(join(FUZZ, "Cargo.toml"))) {
      steps.push(
        step("fuzz-lock", ["metadata", "--format-version", "1", "--offline", "--manifest-path", "fuzz/Cargo.toml"], summaries["fuzz-lock"]!, { runner: runFuzzLock }),
      );
    }
  }
  // Bare, whatever `-p` says: a `-p` build writes a second copy of every workspace crate.
  steps.push(step("build", ["build"], summaries.build!));
  // Whole-workspace runs only from here on for the steps that run `nvs`: a scoped build leaves no
  // binary the tree can trust.
  if (!opts.fast && !opts.package) {
    steps.push(step("nvs-fmt", ["fmt", "<each new or modified .nvs under tests/, examples/>"], summaries["nvs-fmt"]!, { exe: NVS, runner: runNvsFmt }));
  }
  steps.push(step("test", ["test"], summaries.test!, { runner: (s) => runTests(s, opts.package) }));
  if (!opts.fast) {
    if (!opts.package) {
      for (const tree of CASE_TREES) {
        if (existsSync(join(ROOT, "tests", tree))) steps.push(step(tree, ["test", `tests/${tree}`], summaries.cases!, { exe: NVS }));
      }
      steps.push(step("reference", ["nv", "reference"], summaries.reference!, { exe: "bun" }));
    }
    steps.push(step("clippy", ["clippy", "--all-targets", "--", "-D", "warnings"], summaries.clippy!));
    // Last, because it is the one step that is not `cargo` or a script.
    if (!opts.package && existsSync(join(EXTENSION, "package.json"))) {
      steps.push(step("extension", ["run", "--silent", "test:headless"], summaries.extension!, { exe: process.platform === "win32" ? "npm.cmd" : "npm", cwd: EXTENSION }));
    }
  }
  return steps;
}

/** One step's command as a reader types it. */
function shown(s: Step): string {
  let line = [basename(s.exe), ...s.args].join(" ");
  if (s.env) line = Object.entries(s.env).map(([k, v]) => `${k}=${v}`).join(" ") + " " + line;
  if (s.cwd !== ROOT) line += `   (in ${relative(ROOT, s.cwd).replace(/\\/g, "/")})`;
  return line;
}

/** `--list`: the order the gate walks, narrowed by `-p`, `--fast` and `--doc` exactly as a run is. */
function listSteps(opts: Opts): number {
  const steps = stepsFor(opts);
  const scope = opts.package ? ` (-p ${opts.package})` : "";
  console.log(`verify: ${steps.length} step(s) in this order${scope}; \`--list\` runs none of them.`);
  steps.forEach((s, i) => console.log(`  ${i + 1}. ${s.name.padEnd(13)} ${shown(s)}`));
  return 0;
}

// ---- the green cache -------------------------------------------------------------------------------

let graphRead: Promise<Graph | null> | undefined;

/** One reading of every input, or `null` if anything goes wrong, and then nothing is answered from the
 * cache and nothing is recorded in it. The graph is read once: no step changes a manifest. */
async function takeTree(): Promise<Ctx | null> {
  try {
    graphRead ??= metadata().catch(() => null);
    const [tree, graph] = await Promise.all([Tree.read(), graphRead]);
    return { tree, graph, keys: new Map() };
  } catch {
    return null;
  }
}

function keyFor(ctx: Ctx | null, name: string, scope?: string): string | null {
  if (ctx === null) return null;
  const id = `${name}\0${scope ?? ""}`;
  if (!ctx.keys.has(id)) {
    let k: string | null;
    try {
      k = stepKey(ctx.tree, ctx.graph, name, scope);
    } catch {
      k = null;
    }
    ctx.keys.set(id, k);
  }
  return ctx.keys.get(id)!;
}

function loadCache(): Cache {
  const got = readObject<unknown>(CACHE);
  const ok = got.shape === CACHE_SHAPE;
  const obj = (v: unknown) => (ok && v && typeof v === "object" && !Array.isArray(v) ? v : {});
  return { shape: CACHE_SHAPE, steps: obj(got.steps) as Record<string, Entry>, formatted: obj(got.formatted) as Record<string, string> };
}

/** Read the cache, apply `change`, write it back. Read again every time, because two runs overlap by
 * design, a `--doc` beside a `--start`, and each owns only the entries it proved. */
function amendCache(change: (c: Cache) => void): void {
  const cache = loadCache();
  change(cache);
  writeQuiet(CACHE, JSON.stringify(cache, null, 1));
}

/** `-p` narrows which test binaries run and nothing else, so it is part of that one key. */
const scopeOf = (name: string, opts: Opts) => (name === "test" ? opts.package : undefined);

/** The entry `name` was last green under, if the key it has now is that entry's. */
function held(cache: Cache, ctx: Ctx | null, opts: Opts, name: string): Entry | null {
  if (ctx === null || opts.noCache) return null;
  const entry = cache.steps[name];
  if (!entry || typeof entry !== "object" || now() - Number(entry.when ?? 0) > CACHE_TTL_S) return null;
  // An unscoped `test` verdict proves every `-p`; the reverse does not hold.
  const wanted = [keyFor(ctx, name, scopeOf(name, opts)), keyFor(ctx, name)].filter((k) => k !== null);
  return wanted.includes(entry.key ?? "") ? entry : null;
}

function record(ctx: Ctx | null, opts: Opts, name: string, summaryLine: string, seconds: number): void {
  const key = keyFor(ctx, name, scopeOf(name, opts));
  if (key === null) return;
  const entry: Entry = { key, when: now(), summary: summaryLine, seconds: Math.round(seconds * 10) / 10 };
  amendCache((c) => (c.steps[name] = entry));
}

// ---- output ----------------------------------------------------------------------------------------

function clock(seconds: number): string {
  if (seconds >= 60) return `${Math.floor(seconds / 60)}m${String(Math.floor(seconds) % 60).padStart(2, "0")}s`;
  return `${seconds.toFixed(0)}s`;
}

const ago = (seconds: number) => (seconds < 90 ? `${Math.floor(seconds)}s ago` : `${Math.floor(seconds / 60)}m ago`);

function tail(text: string, limit: number): [string, number] {
  const lines = text.replace(/\n+$/, "").split("\n");
  if (limit <= 0 || lines.length <= limit) return [lines.join("\n"), 0];
  return [lines.slice(-limit).join("\n"), lines.length - limit];
}

/** Rewrites `PROGRESS`: the step about to run, or, with `finished`, the verdict. */
function progress(done: Step[], s?: Step, total = 0, finished?: number, index?: number): void {
  const entry: Record<string, unknown> = { pid: process.pid, at: now(), done: done.map((d) => ({ name: d.name, seconds: Math.round(d.seconds * 10) / 10 })) };
  if (s) Object.assign(entry, { step: s.name, index: index ?? done.length + 1, total });
  if (finished !== undefined) entry.finished = finished;
  writeQuiet(PROGRESS, JSON.stringify(entry));
}

/** Says once, and never fails a run, when this clone has not enabled the versioned hooks. */
async function hooksNote(): Promise<void> {
  let got = "";
  try {
    got = (await proc(["git", "config", "core.hooksPath"])).stdout.trim();
  } catch {
    return;
  }
  if (got.replace(/\\/g, "/").replace(/\/+$/, "") === "tools/git-hooks") return;
  console.log("verify: this clone has no hooks -- `git config core.hooksPath tools/git-hooks`");
  console.log("        (it rejects attribution trailers; docs/agent/conventions.md says why)\n");
}

// ---- --start and --wait ----------------------------------------------------------------------------

/** Starts the same verification as an `nv bg` job and returns at once, so the wrap is written while it
 * runs. It is the same steps, the same cache and the same exit status. */
function startBackground(args: string[]): number {
  const passthrough = args.filter((a) => a !== "--start" && a !== "--wait");
  const id = startJob([process.execPath, join(ROOT, "tools", "nv", "main.ts"), "verify", ...passthrough]);
  writeQuiet(BACKGROUND, JSON.stringify({ id }));
  console.log(`verify: started in the background (job ${id}).`);
  console.log("        Write the wrap file now, then `bun nv verify --wait` to collect it.");
  return 0;
}

/** Collects a `--start` run: its whole output, and its own exit status as this call's. */
async function waitBackground(): Promise<number> {
  const id = (readJson(BACKGROUND) as { id?: string } | undefined)?.id;
  const job = id ? readJob(id) : null;
  if (!id || job === null) {
    console.log("verify: nothing was started. `bun nv verify --start` first, or just run");
    console.log("        `bun nv verify` -- there is no state to recover here.");
    return 2;
  }
  const began = performance.now();
  let code = exitOf(id);
  while (code === null) {
    if (!alive(job.pid)) {
      code = exitOf(id);
      if (code !== null) break;
      console.log(`verify: the background run ended with no exit status. Its output is in .agent-tmp/bg/${id}/log.`);
      return 1;
    }
    if ((performance.now() - began) / 1000 > BACKGROUND_TIMEOUT_S) {
      console.log(`verify: the background run has not finished after ${BACKGROUND_TIMEOUT_S}s.`);
      console.log(`        Its output so far is in .agent-tmp/bg/${id}/log.`);
      return 2;
    }
    await Bun.sleep(400);
    code = exitOf(id);
  }
  const log = join(dirOf(id), "log");
  const body = existsSync(log) ? readFileSync(log, "utf8").replace(/\r\n/g, "\n") : "";
  process.stdout.write(body.endsWith("\n") || !body ? body : body + "\n");
  const waited = (performance.now() - began) / 1000;
  if (waited >= 1) console.log(`-- collected after a further ${waited.toFixed(0)}s; the rest of the run overlapped whatever you did in between.`);
  return code;
}

// ---- the run ---------------------------------------------------------------------------------------

async function verify(opts: Opts): Promise<number> {
  const steps = stepsFor(opts);
  const scope = opts.package ? ` (-p ${opts.package})` : "";
  await hooksNote();

  const cache = loadCache();
  let ctx = await takeTree();
  for (const s of steps) if (s.name === "nvs-fmt") [s.todo, s.changed] = await unformatted(cache);

  const answered = (s: Step): Entry | null => {
    if (s.name === "nvs-fmt") return s.todo!.length ? null : { summary: "nothing new to format", seconds: 0 };
    return held(cache, ctx, opts, s.name);
  };

  const done: Step[] = [];
  const unchanged = new Map<string, Entry>();
  let failed: Step | null = null;
  // `fmt` red: reported only if nothing after it is.
  let deferred: Step | null = null;
  const began = performance.now();

  /** `null` when `s` has to run; otherwise the green entry that replaces running it. */
  const needed = (i: number, s: Step): Entry | null => {
    let entry = answered(s);
    if (entry !== null && s.name === "build") {
      // Only if nothing after it will use what `build` leaves, and `test` does not when its binaries
      // are provably the ones this code compiles to.
      const leansOnBuild = (t: Step) => NEEDS_BINARY.has(t.name) && answered(t) === null && (t.name !== "test" || jobsOnDisk(keyFor(ctx, "build")) === null);
      if (steps.slice(i + 1).some(leansOnBuild)) entry = null;
    }
    if (entry !== null) unchanged.set(s.name, entry);
    return entry;
  };

  const prepare = (i: number, s: Step) => {
    if (s.name === "test") {
      // `--no-cache` is every step for real, and cargo's build is part of this one.
      s.binaryKey = !opts.noCache ? keyFor(ctx, "build") : null;
      s.ctx = ctx;
      s.noCache = opts.noCache;
      if (!opts.package) s.skipDoc = held(cache, ctx, opts, "test:doc") !== null;
    }
    progress(done, s, steps.length, undefined, i + 1);
  };

  /** One finished run's verdict, taken in list order; false when it stops the run. */
  const settle = async (s: Step, ok: boolean): Promise<boolean> => {
    if (WRITES.has(s.name) && s.out.trim()) {
      // It rewrote files, so every verdict from here on belongs to the tree as it now is.
      ctx?.tree.save();
      ctx = await takeTree();
    }
    if (s.formatted && Object.keys(s.formatted).length) {
      // A path that is no longer new or modified has been committed, and is dropped.
      const kept = new Set(s.changed ?? []);
      amendCache((c) => (c.formatted = { ...Object.fromEntries(Object.entries(c.formatted).filter(([k]) => kept.has(k))), ...s.formatted }));
    }
    if (s.docGreen) record(ctx, opts, "test:doc", "ok", 0);
    if (s.name === "fmt" && !ok) {
      deferred = s;
      return true;
    }
    if (!ok) {
      failed = s;
      return false;
    }
    record(ctx, opts, s.name, s.summarize(s.out), s.seconds);
    done.push(s);
    return true;
  };

  /** The script steps from `i` on, run at the same time as the `build` after them. */
  const besideBuild = async (i: number): Promise<number> => {
    let j = i;
    while (j < steps.length && BESIDE_BUILD.has(steps[j]!.name)) j++;
    const group = j < steps.length && steps[j]!.name === "build" ? steps.slice(i, j + 1) : steps.slice(i, j);
    const todo = group.map((s, k) => [i + k, s] as const).filter(([k, s]) => needed(k, s) === null);
    const running = todo.map(([k, s]) => {
      prepare(k, s);
      return [s, runStep(s)] as const;
    });
    for (const [s, p] of running) if (!(await settle(s, await p))) break;
    await Promise.allSettled(running.map(([, p]) => p));
    return group.length;
  };

  /** `test`, and the steps after it, started on one lane while its slowest binaries still run. */
  const withTail = async (i: number): Promise<number> => {
    const test = steps[i]!;
    if (needed(i, test) !== null) return 1;
    const after = steps
      .slice(i + 1)
      .map((s, k) => [i + 1 + k, s] as const)
      .filter(([k, s]) => needed(k, s) === null);
    let release!: () => void;
    const tailReached = new Promise<void>((r) => (release = r));
    let stop = false;
    const ran = new Map<string, boolean>();
    const lane = (async () => {
      await tailReached;
      for (const [k, s] of after) {
        if (stop) return;
        prepare(k, s);
        const ok = await runStep(s);
        ran.set(s.name, ok);
        if (!ok) return;
      }
    })();
    test.onTail = release;
    // A failed binary is run again alone, and alone means nothing on the lane either.
    test.quiesce = async () => {
      stop = true;
      release();
      await lane;
    };
    prepare(i, test);
    const ok = await runStep(test);
    if (!ok) stop = true;
    release();
    await lane;
    if (await settle(test, ok)) {
      for (const [, s] of after) if (!ran.has(s.name) || !(await settle(s, ran.get(s.name)!))) break;
    }
    return steps.length - i;
  };

  let i = 0;
  while (i < steps.length && failed === null) {
    const s = steps[i]!;
    if (BESIDE_BUILD.has(s.name) || s.name === "build") i += await besideBuild(i);
    else if (s.name === "test") i += await withTail(i);
    else {
      if (needed(i, s) === null) {
        prepare(i, s);
        await settle(s, await runStep(s));
      }
      i++;
    }
  }
  const stopped = failed as Step | null;
  // A step after the one that stopped the run was not reached, whatever the cache holds.
  if (stopped !== null) for (const s of steps.slice(steps.indexOf(stopped) + 1)) unchanged.delete(s.name);
  const red: Step | null = stopped ?? (deferred as Step | null);
  progress(done, undefined, 0, red ? 1 : 0);
  (ctx as Ctx | null)?.tree.save();

  const total = (performance.now() - began) / 1000;
  const lines = () => {
    for (const s of steps) {
      const e = unchanged.get(s.name);
      if (e) console.log(`  ${s.name.padEnd(13)} ${"--".padStart(6)}   ${e.summary ?? "ok"}`);
      else if (done.includes(s)) console.log(`  ${s.name.padEnd(13)} ${clock(s.seconds).padStart(6)}   ${s.summarize(s.out)}`);
    }
  };

  if (red === null && done.length === 0) {
    const whens = [...unchanged.values()].filter((e) => e.when !== undefined).map((e) => Number(e.when));
    const oldest = whens.length ? Math.min(...whens) : now();
    console.log(`verify: green, nothing a step reads has changed since ${ago(now() - oldest)} -- nothing to re-run${scope}`);
    lines();
    const cost = [...unchanged.values()].reduce((n, e) => n + Number(e.seconds ?? 0), 0);
    console.log(`\nthat verdict cost ${clock(cost)} and every step's inputs are as they were; \`--no-cache\` runs it again anyway.`);
    return 0;
  }

  const note = unchanged.size ? ` -- ${unchanged.size} not re-run, their inputs unchanged (\`--\` below)` : "";
  if (red === null) {
    console.log(`verify: ${done.length + unchanged.size} of ${steps.length} green in ${clock(total)}${scope}${note}`);
    lines();
    return 0;
  }

  amendCache((c) => delete c.steps[red.name]);
  console.log(`verify: FAILED at ${red.name} (step ${steps.indexOf(red) + 1} of ${steps.length}) after ${clock(total)}${scope}${note}`);
  lines();
  console.log(`  ${red.name.padEnd(13)} ${"---".padStart(6)}   exit ${red.code}`);
  const [body, hidden] = tail(red.out, opts.full ? 0 : TAIL_LINES);
  console.log(`\n-- \`${cmdOf(red)}\`` + (hidden ? `, last ${TAIL_LINES} lines` : ""));
  console.log(body);
  if (hidden) console.log(`\n... ${hidden} earlier line(s) hidden`);
  console.log(`\nfull output: .agent-tmp/verify-${red.name}.log`);
  return 1;
}

/** `--keys`: `{name: {key, wide}}` for every binary the last build recorded, keyed as `test` keys it. */
async function printKeys(): Promise<number> {
  const ctx = await takeTree();
  if (!ctx?.graph) {
    console.error("nv verify: the tree or `cargo metadata` could not be read, and every key needs both");
    return 2;
  }
  const built = readJson(TEST_BUILT) as { jobs?: Job[] } | undefined;
  const reach = new Reach(ctx, ctx.graph);
  const out: Record<string, { key: string; wide: boolean }> = {};
  for (const j of built?.jobs ?? []) out[j.name] = { key: reach.key(j), wide: reach.wide(j) };
  console.log(JSON.stringify(out));
  return 0;
}

const USAGE = [
  "usage: nv verify [-h] [-p PACKAGE] [--fast] [--doc] [--full] [--no-cache]",
  "                 [--start] [--wait] [--list] [--keys]",
].join("\n");

export async function run(args: string[]): Promise<number> {
  let parsed;
  try {
    parsed = parseArgs(args, { flags: ["--fast", "--doc", "--full", "--no-cache", "--start", "--wait", "--list", "--keys"], valued: ["--package"], short: { "-p": "--package" } });
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv verify: error: ${e.message}`);
    return 2;
  }
  const { flags, values } = parsed;
  if (flags.has("--help")) {
    console.log(`${USAGE}\n\nnv verify: ${summary}\n\nThe module doc of tools/nv/cmd/verify.ts says what each step is and why.`);
    return 0;
  }
  if (flags.has("--keys")) {
    if (flags.size > 1 || values.size > 0) {
      console.log("verify: --keys runs nothing, and takes no other flag.");
      return 2;
    }
    return printKeys();
  }
  const opts: Opts = {
    ...(values.has("--package") ? { package: values.get("--package")! } : {}),
    fast: flags.has("--fast"),
    doc: flags.has("--doc"),
    full: flags.has("--full"),
    noCache: flags.has("--no-cache"),
    start: flags.has("--start"),
    wait: flags.has("--wait"),
    list: flags.has("--list"),
  };
  if (opts.start && opts.wait) {
    console.log("verify: --start and --wait are two calls, not one flag pair.");
    return 2;
  }
  if (opts.doc && opts.fast) {
    console.log("verify: --doc is a run of one step; --fast has nothing to narrow.");
    return 2;
  }
  if (opts.list) {
    if (opts.start || opts.wait) {
      console.log("verify: --list runs nothing, so there is nothing to --start or --wait for.");
      return 2;
    }
    return listSteps(opts);
  }
  if (opts.start) return startBackground(args);
  if (opts.wait) return waitBackground();
  return verify(opts);
}
