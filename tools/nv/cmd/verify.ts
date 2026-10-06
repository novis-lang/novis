// `bun nv verify`: the build gate, one command. A loop session runs it as AGENTS.md § *Session workflow*
// step 3; anywhere else `bun nv affected --run` runs it first and then the acceptance checks a change
// reaches.
//
//     bun nv verify                  every step the change reaches
//     bun nv verify -p nvs-ir        the same build; only nvs-ir's test binaries run
//     bun nv verify --fast           build and test only, for a mid-work check
//     bun nv verify --doc            the rustdoc gate alone; the driver's goal-end call
//     bun nv verify --start          run it detached through `nv bg` and return at once
//     bun nv verify --wait           collect what --start left, with its exit status
//     bun nv verify --full           do not truncate the failing step's output
//     bun nv verify --no-cache       run every step, every test binary and every case, whatever changed
//     bun nv verify --list           the steps in order, running none of them
//
// The steps, in order, stopping at the first failure: `cargo fmt`, `bun nv lints --check`,
// `bun nv directives --check` and `--check-template`, `bun nv owners --check`, `bun nv selftest`,
// the fuzz workspace's lock brought back in step, `cargo build`, `nvs fmt` over the `.nvs` files this
// working tree added or changed, `cargo test`, the `.nvst` trees, `bun nv reference`, `cargo clippy
// --all-targets -- -D warnings`, and the VS Code extension's headless suites. Green prints one line per
// step; a failure prints that step's output and nothing else. The full output of every step is written
// to `.agent-tmp/verify-<step>.log`. This command judges nothing: a step's own exit status is the whole
// verdict.
//
// Every build and every run is the `covws` build (`lib/covws.ts`): `target/covws`, the workspace's own
// crates compiled with coverage counters. `cargo build`, `cargo test`, `cargo doc` and the doc-tests go
// through the `covwrap` wrapper with `--target <host>`; `cargo clippy` shares the directory without it,
// since clippy is a workspace wrapper itself. The test binaries, the case trees, `reference` and the
// extension run that build's `nvs`.
//
// **What runs is what the change reaches** (`tools/nv/select/`). The change is every path that differs
// from the tree the store last recorded, and each atom whose recorded footprint holds a key the change
// moved is selected: a test binary, a case of the two trees, and each step as one atom. A step that
// executes no code of ours is keyed on the files it reads: `fmt` on every Rust file; `build`, `clippy`,
// `doc` and the doc-tests on every file the dep-info cargo wrote names, and each build script's inputs;
// the `bun nv` steps on what they were seen to read (`lib/reads.ts`); the `nv` step's `tsc` on every
// module under `tools/nv`, and each of its `bun test` files, run one process per file, as an atom of
// its own on what that file was seen to read (`--preload`); `fuzz-lock` on the fuzz workspace's
// manifest and lock; the extension on its own
// directory. Whatever a step runs of `nvs` is recorded by coverage as well. `build` runs when it is
// selected or when a later step that uses the binary runs. A step that is not selected is not started,
// and prints `--` with the summary of its last green run. `--no-cache` selects everything.
//
// Every run records: each atom that ran records its footprint and verdict in the store, green or red.
// The run then moves the store's tree to the one it was selected against (`record.ts` `advance`),
// whether it was green or red: every atom the change selected that this run did not run is marked
// owed first, so a proof program, another case tree or a step `--fast` skipped stays selected for
// whoever runs it next, and a red atom stays selected as red until a run of it is green. So the run
// after one red test repeats that test, not the whole of the change's reach.
//
// `fmt` and `nvs-fmt` format rather than check, because a red `fmt` was only ever fixed by running the
// formatter and verifying again. `fmt` runs first, so everything after it compiles the text the commit
// carries, and its own failure is reported only when every other step passed: a parse error reads
// better from `build`. `nvs-fmt` runs over only the new or modified `.nvs` files under `tests/` and
// `examples/`, `tests/fmt/input/` excepted, that it has not formatted as they now stand, so a layout
// rule that changes what the formatter prints is still `crates/nvs-fmt/tests/identity.rs`'s to judge
// over the rest. A file the formatter cannot parse is left as it was, and the step is never red. After
// either one rewrites a file the change is read again, and an atom that ran before the rewrite and is
// selected again counts as not run.
//
// `test` builds the selected binaries with `cargo test --no-run` and the target filters that name them
// (`--lib`, `--bin <name>`, `--test <name>`), never a `-p`, so a change to a library every binary links
// relinks only the binaries that run; the workspace graph names the rest. Without a graph, and under
// `--no-cache`, it builds every one. It runs them side by side, as many at a time as there are cores,
// the slowest of the last run first, each with libtest threads in proportion to its last time, and the
// doc-tests beside them when they are selected. A binary that fails is run a second time, alone: a
// binary that passes alone shares a port, a path or a container with another, and is reported red with
// that diagnosis rather than retried into green. A binary not selected prints the `test result:` line
// of its last green run. Each binary runs with a compile cache and a temporary directory of its own, and
// every `nvs` it starts records into its footprint. An unscoped run fails on a wide binary among those
// it built that `tools/data/impact-wide.txt` does not list, and on a listed one that is narrow now
// (`tools/nv/keys/escape.ts`), from the paths its footprint says it asked `nvs_repo` for.
//
// Two stretches run at the same time and are still judged in list order. The script steps run beside
// `build`: none needs what it writes, except a few of `selftest`'s bun tests that run the `covws` `nvs`,
// so in a tree where that binary does not exist yet `selftest` moves to just after `build`. Once every
// test binary has started and at most half the cores are busy, the steps after `test` start on one
// lane, one at a time; nothing starts there after a binary has failed, and the lane stops before a
// failed binary is run again alone. A red `test` drops the lane's verdicts.
//
// `-p` narrows which test binaries run, off the one build, and the steps that run `nvs` over the trees
// do not run; what it leaves out is owed. `cargo doc` with broken intra-doc links denied is `--doc`, run
// alone. The documentation gates (`rules`, `records`, links, the plan, the playbook) are not steps: `nv
// session --wrap` refuses a wrap that breaks them.
//
// Every process verify starts is killed with the processes it started when it runs out of time, and a
// test binary or case batch that fails has what it left running killed (`lib/proc.ts`).
//
// `.agent-tmp/verify-progress.json` names the step in flight, for the loop driver's status line.
//
// Exits 0 when every step is green, 1 when one is red, and 2 on a bad argument or a `--wait` with nothing
// to collect.

import { cpus } from "node:os";
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative } from "node:path";

import { COVWS_TARGET, covwsCargo, covwsNvs, hostTriple } from "../lib/covws.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";
import { cargoStatus, progress as showProgress } from "../lib/progress.ts";
import { ArgError, parseArgs } from "../lib/py.ts";

import { findings } from "../keys/escape.ts";
import { type Graph, metadata, TARGET_FLAGS, targetFilters } from "../keys/graph.ts";
import { digest } from "../keys/scan.ts";
import { recordName } from "../proofs/run.ts";
import { caseFiles, caseId, nvTestFiles, nvTestId } from "../select/atoms.ts";
import { buildScripts } from "../select/build.ts";
import { WILD, repoPath } from "../select/keys.ts";
import { advance, depInfoPaths, fullChange, pool, putTestGreen, Recorder, recordCases, testGreen } from "../select/record.ts";
import { type ChangeSet, computeChange, discover, type Selection, query, rustFiles } from "../select/select.ts";
import { NV_TSC, runNvTest, runTsc } from "../select/nvtests.ts";
import { nvKeys, testKeys } from "../select/seed.ts";
import { type Keyed, SelectStore, testReads, type Verdict } from "../select/store.ts";
import { alive, dirOf, exitOf, readJob, startJob } from "./bg.ts";

export const summary = "the gate, one call: nv verify [-p <crate>] [--fast] [--doc] [--start | --wait] [--list] [--full] [--no-cache]";

const TMP = join(ROOT, ".agent-tmp");
const PROGRESS = join(TMP, "verify-progress.json");
/** Each test binary's seconds in the last run, so the next one starts the slowest first. */
const TEST_TIMES = join(TMP, "verify-test-times.json");
/** The job id of the last `--start`. */
const BACKGROUND = join(TMP, "verify-background.json");
const BACKGROUND_TIMEOUT_S = 600;

const TAIL_LINES = 60;
/** A step is killed past this, so a hung test cannot hold a session forever. */
const STEP_TIMEOUT_MS = 60 * 60 * 1000;

/** The steps that use what `build` leaves on disk. */
const NEEDS_BINARY = new Set(["nvs-fmt", "test", "conformance", "reference", "extension"]);
/** Whether step `name`, listed after `build`, uses what it leaves on disk: a script step is listed there
 * only when it does and the binary is not built yet (`stepsFor`). */
const usesBinary = (name: string) => NEEDS_BINARY.has(name) || BESIDE_BUILD.has(name);
/** The steps that rewrite source, after which the change is read again. */
const WRITES = new Set(["fmt", "nvs-fmt"]);
/** The script steps, which run at the same time as `build` when they are listed before it. */
const BESIDE_BUILD = new Set(["lints", "directives", "template", "owners", "nv", "fuzz-lock"]);
const NVS_FMT_TREES = ["tests", "examples"];
const NVS_FMT_SKIPS = "tests/fmt/input/";
const EXTENSION = join(ROOT, "editors", "vscode");
const FUZZ = join(ROOT, "fuzz");
/** The `.nvst` trees verify runs. */
const CASE_TREES = ["conformance"];
const EXE = process.platform === "win32" ? ".exe" : "";
/** The doc-tests, one atom: rustdoc builds and runs each in a directory it deletes. */
const DOC_TESTS = "step:doc-tests";
/** Every step's atom, whether or not this run's options walk it. */
const STEP_ATOMS = ["fmt", "lints", "directives", "template", "owners", "nv", "fuzz-lock", "build", "reference", "clippy", "extension", "doc"].map((n) => `step:${n}`).concat(DOC_TESTS);

const RESULT_RE = /test result: \w+\. (\d+) passed; (\d+) failed/g;
const CASES_RE = /(\d+) passed, (\d+) failed, (\d+) skipped/g;
const WARN_RE = /^(warning|error)(\[[^\]]+\])?: (.*)$/gm;

/** The `covws` build's directory of binaries. */
const covDir = () => abs(`${COVWS_TARGET}/${hostTriple()}/debug`);

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
  /** The step's own atom, recorded after each run with the keys `footprint` gives. */
  atom?: string;
  footprint?: (s: Step) => Promise<Keyed>;
  seconds: number;
  code: number | null;
  out: string;
  // `nvs-fmt`'s state: what it still has to format, every changed file, and each digest it left.
  todo?: string[];
  changed?: string[];
  formatted?: Record<string, string>;
  onTail?: () => void;
  quiesce?: () => Promise<void>;
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

/** Best effort: a file under `.agent-tmp` that cannot be written costs the next run its ordering. */
function writeQuiet(path: string, text: string): void {
  try {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, text);
  } catch {
    // See above.
  }
}

// ---- the selection ---------------------------------------------------------------------------------

/** What this run was asked, the store, and the atoms the change selects. */
interface Run {
  opts: Opts;
  store: SelectStore;
  graph: Graph | null;
  change: ChangeSet;
  sel: Selection;
  /** The test binaries the workspace graph names, by `<package> <kind> <target>`. */
  testNames: Set<string>;
  rec: Recorder;
  /** Every atom this run ran and recorded. */
  ran: Set<string>;
}

/** The change since the store's tree and what it selects. A store with no tree, or a tree git can no
 * longer name, selects everything. */
async function select(store: SelectStore, graph: Graph | null): Promise<{ change: ChangeSet; sel: Selection; testNames: Set<string> }> {
  let change: ChangeSet;
  if (store.base() === null) change = await fullChange();
  else {
    try {
      change = await computeChange(store, { graph });
    } catch (e) {
      change = await fullChange(ROOT, `the recorded tree could not be read: ${(e as Error).message.split("\n")[0]}`);
    }
  }
  const found = discover(graph);
  const testNames = new Set(found.atoms.filter((id) => id.startsWith("test:")).map((id) => id.slice(5)));
  const sel = query(store, change, { discovered: [...found.atoms, ...STEP_ATOMS], complete: found.complete });
  return { change, sel, testNames };
}

const picked = (r: Run, id: string) => r.opts.noCache || r.sel.selected.has(id);

/** The test binaries this run runs, by name. */
function testsToRun(r: Run): string[] {
  const names = [...r.testNames].filter((n) => !r.opts.package || n.startsWith(`${r.opts.package} `));
  return names.filter((n) => picked(r, `test:${n}`)).sort();
}

/** The cases of `tests/<tree>` this run runs. */
function casesToRun(r: Run, tree: string): string[] {
  const prefix = `tests/${tree}/`;
  if (r.opts.noCache) return caseFiles().filter((p) => p.startsWith(prefix));
  return [...r.sel.selected.keys()].filter((id) => id.startsWith(`case:${prefix}`)).map((id) => id.slice(5)).sort();
}

/** Whether step `s` has anything to run, and why, before `build`'s dependence on the steps after it. */
function wanted(r: Run, s: Step): boolean {
  if (s.name === "nvs-fmt") return (s.todo ?? []).length > 0;
  if (s.name === "nv") return picked(r, NV_TSC) || nvTestsToRun(r).length > 0;
  if (s.name === "test") return testsToRun(r).length > 0 || (!r.opts.package && picked(r, DOC_TESTS));
  if (CASE_TREES.includes(s.name)) return casesToRun(r, s.name).length > 0;
  return s.atom !== undefined && picked(r, s.atom);
}

// ---- running ---------------------------------------------------------------------------------------

/** Each running step's name and what it is doing now; `lib/progress.ts` shows them while they run. */
const running = new Map<string, string>();

/** Sets what the step `name` is doing now, while it runs. */
function stepDetail(name: string, detail: string): void {
  if (!running.has(name)) return;
  running.set(name, detail);
  showProgress(`verify: ${[...running].map(([n, d]) => (d ? `${n} (${d})` : n)).join(", ")}`);
}

/** A `proc.run` `onLine` that shows cargo's status lines as the step's detail. */
function cargoDetail(name: string): (line: string) => void {
  const status = cargoStatus();
  return (line) => {
    const said = status(line);
    if (said !== null) stepDetail(name, said);
  };
}

async function spawnOut(argv: string[], cwd: string, env?: Record<string, string>, onLine?: (line: string) => void, reap = false): Promise<[number, string]> {
  try {
    const p = await proc(argv, { cwd, timeoutMs: STEP_TIMEOUT_MS, reap, ...(env ? { env } : {}), ...(onLine ? { onLine } : {}) });
    return [p.code, p.stdout + p.stderr + (p.timedOut ? `\nkilled after ${STEP_TIMEOUT_MS / 60000} minutes\n` : "")];
  } catch (e) {
    return [-1, `could not run \`${argv.join(" ")}\`: ${(e as Error).message}`];
  }
}

async function runStep(s: Step): Promise<boolean> {
  const started = performance.now();
  running.set(s.name, "");
  stepDetail(s.name, "");
  const onLine = s.exe === "cargo" ? cargoDetail(s.name) : undefined;
  try {
    [s.code, s.out] = s.runner ? await s.runner(s) : await spawnOut([s.exe, ...s.args], s.cwd, s.env, onLine, true);
  } finally {
    running.delete(s.name);
  }
  s.seconds = (performance.now() - started) / 1000;
  writeQuiet(join(TMP, `verify-${s.name}.log`), s.out);
  return s.code === 0;
}

// ---- footprints of the steps that execute no code of ours ------------------------------------------

/** Every file the `covws` build's dep-info names, and each build script's inputs: what a compile of the
 * workspace reads. */
function buildKeys(graph: Graph | null): Keyed {
  const keys: Keyed = new Map();
  for (const p of depInfoPaths(join(covDir(), "deps"), (x) => repoPath(x))) keys.set(`file:${p}`, "");
  const pkgDirs = new Map([...(graph?.values() ?? [])].map((p) => [p.name, p.dir]));
  for (const script of buildScripts(join(covDir(), "build"), pkgDirs).values()) for (const input of script.inputs) keys.set(`tree:${input}`, "");
  // A build with no dep-info to read is keyed on everything.
  if (keys.size === 0) keys.set(WILD, "");
  return keys;
}

/** A configuration file a tool reads when it is there. */
const maybe = (keys: Keyed, path: string) => {
  keys.set(`file:${path}`, "");
  keys.set(`exists:${path}`, "");
};

async function fmtKeys(): Promise<Keyed> {
  const keys: Keyed = new Map();
  for (const f of await rustFiles()) keys.set(`file:${f}`, "");
  maybe(keys, "rustfmt.toml");
  maybe(keys, ".rustfmt.toml");
  return keys;
}

// ---- the test step ---------------------------------------------------------------------------------

/** `cargo test --no-run` on `covws`, as the job of each test binary it built, or `[null, output]` when
 * it fails. With `filters` it builds only the targets they name, and with none nothing at all; with
 * `null` it builds every workspace test binary. A `-p` never goes on the build, since it would resolve
 * features over one package and build a second copy of the workspace. */
async function buildTestJobs(filters: string[] | null): Promise<[Job[] | null, string]> {
  if (filters !== null && filters.length === 0) return [[], ""];
  const { env, args } = covwsCargo();
  let p;
  try {
    p = await proc(["cargo", "test", "--no-run", ...args, ...(filters ?? []), "--message-format=json-render-diagnostics"], { env, timeoutMs: STEP_TIMEOUT_MS, onLine: cargoDetail("test") });
  } catch (e) {
    return [null, `could not run \`cargo test --no-run\`: ${(e as Error).message}`];
  }
  if (p.code !== 0) return [null, p.stderr + p.stdout];
  const jobs: Job[] = [];
  for (const line of p.stdout.split("\n")) {
    let m: {
      reason?: string;
      package_id?: string;
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
    if (m.reason !== "compiler-artifact" || !m.executable || !m.profile?.test) continue;
    const id = m.package_id ?? "";
    const hash = id.lastIndexOf("#");
    const source = hash < 0 ? "" : id.slice(0, hash);
    const tail = hash < 0 ? id : id.slice(hash + 1);
    const owner = tail.includes("@") ? tail.split("@")[0]! : source.replace(/\/+$/, "").split("/").pop()!;
    const target = m.target.name;
    const kind = (m.target.kind ?? []).find((k) => k in TARGET_FLAGS) ?? "lib";
    const cwd = dirname(m.manifest_path);
    // The reproduction stays inside the same build: a target flag narrows the run, a `-p` would not.
    const rerun = kind === "lib" ? `bun nv verify -p ${owner}` : `cargo test ${TARGET_FLAGS[kind]} ${target}`;
    jobs.push({ name: `${owner} ${kind} ${target}`, owner, argv: [m.executable], cwd, env: { CARGO_MANIFEST_DIR: cwd }, rerun });
  }
  return [jobs, ""];
}

async function runJob(job: Job): Promise<[number, number, string]> {
  const started = performance.now();
  const [code, out] = await spawnOut(job.argv, job.cwd, job.env, undefined, true);
  return [(performance.now() - started) / 1000, code, out];
}

const RERUN_ALONE_PASSED = (name: string) =>
  `error: \`${name}\` failed beside the other test binaries and passed alone. It shares something with a ` +
  `binary that runs at the same time -- a fixed port, a fixed path under the shared temp or target directory, ` +
  `a container name -- or leans on a timeout that load breaks. Give the test its own (port 0, a directory no ` +
  `other test names, its own container) rather than running it apart: \`tools/nv/cmd/verify.ts\`'s module doc.`;

/** The `test` step: the selected binaries and, when selected, the doc-tests. */
async function runTests(r: Run, s: Step): Promise<[number, string]> {
  const pkg = r.opts.package;
  const chosen = new Set(testsToRun(r));
  // With the graph, every test binary is named without a build, so only the chosen ones are built.
  // Without it, or with `--no-cache`, every one is built and every one the graph does not name runs.
  const narrow = r.graph !== null && !r.opts.noCache;
  const [all, note] = await buildTestJobs(narrow ? targetFilters([...chosen]) : null);
  if (all === null) return [1, note];
  const nvs = covwsNvs();
  const exists = narrow ? r.testNames : new Set(all.map((j) => j.name));
  // A test atom whose binary the workspace no longer has is gone.
  if (!pkg) for (const a of r.store.atoms("test")) if (!exists.has(a.id.slice(5))) r.store.removeAtom(a.id);
  const ownerOf = (name: string) => name.split(" ")[0]!;
  const scoped = [...exists].filter((n) => !pkg || ownerOf(n) === pkg);
  let jobs = all.filter((j) => (!pkg || j.owner === pkg) && (r.opts.noCache || chosen.has(j.name) || !r.testNames.has(j.name)));
  const docs = !pkg && picked(r, DOC_TESTS);
  const docDir = join(r.rec.dir, "doctests");
  if (docs) {
    const { env, args } = covwsCargo();
    mkdirSync(docDir, { recursive: true });
    // Each doc-test's process writes its counters here rather than beside the crate it tests.
    jobs.push({ name: "doc-tests", owner: "", argv: ["cargo", "test", "--doc", ...args], cwd: ROOT, env: { ...env, LLVM_PROFILE_FILE: join(docDir, "d-%4m.profraw") }, rerun: "cargo test --doc" });
  }
  const last = readObject<number>(TEST_TIMES);

  for (const j of jobs) {
    if (j.name === "doc-tests") continue;
    const name = recordName(`test:${j.name}`);
    j.env = { ...j.env, ...r.rec.env(name), ...(await r.rec.cacheDir(name)) };
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

  // Each binary's footprint is taken as soon as it ends, beside the binaries still running.
  const recording: Promise<void>[] = [];
  const recordJob = (j: Job, code: number, text: string) => {
    if (j.name === "doc-tests") {
      rmSync(docDir, { recursive: true, force: true });
      r.store.recordRun(DOC_TESTS, { def: digest(j.argv.join(" ")), verdict: code === 0 ? "green" : "red", keys: buildKeys(r.graph) });
      r.ran.add(DOC_TESTS);
      return;
    }
    const id = `test:${j.name}`;
    recording.push(
      r.rec.keysOf(recordName(id), [j.argv[0]!, nvs]).then((ext) => {
        const verdict: Verdict = code === 0 && ext ? "green" : "red";
        r.store.recordRun(id, { def: "", verdict, keys: testKeys(ext, j.name) });
        r.ran.add(id);
        putTestGreen(r.store, j.name, verdict, text);
      }),
    );
  };

  // The tail: every job started and at most half the cores still running one.
  const results = new Map<string, [number, number, string]>();
  let left = jobs.length;
  let red = false;
  let next = 0;
  const count = () => stepDetail(s.name, `${jobs.length - left}/${jobs.length} test binaries${red ? ", one failed" : ""}`);
  count();
  const worker = async () => {
    while (next < jobs.length) {
      const j = jobs[next++]!;
      const got = await runJob(j);
      results.set(j.name, got);
      recordJob(j, got[1], got[2]);
      left--;
      red ||= got[1] !== 0;
      count();
      if (s.onTail && !red && left <= Math.floor(workers / 2)) s.onTail();
    }
  };
  await Promise.all(Array.from({ length: Math.min(workers, jobs.length) }, worker));
  if (s.onTail && jobs.length === 0) s.onTail();

  const failed = jobs.filter((j) => results.get(j.name)![1] !== 0);
  // Alone, one at a time, after the pool has drained and the lane has stopped.
  if (failed.length && s.quiesce) await s.quiesce();
  await Promise.all(recording);
  const alone = new Map<string, boolean>();
  for (const j of failed) {
    if (j.name !== "doc-tests") j.env = { ...j.env, ...r.rec.env(recordName(`test:${j.name}`)), ...(await r.rec.cacheDir(recordName(`test:${j.name}`))) };
    else mkdirSync(docDir, { recursive: true });
    const [, code] = await runJob(j);
    alone.set(j.name, code === 0);
    // What the second run left is read and dropped: the verdict is the first run's.
    if (j.name !== "doc-tests") await r.rec.keysOf(recordName(`test:${j.name}`), [j.argv[0]!, nvs]);
    else rmSync(docDir, { recursive: true, force: true });
  }

  let wide: string[] = [];
  // Unscoped runs only: the wide list is the workspace's, and `-p` sees one package's.
  if (!pkg && r.graph) {
    const reads = testReads(r.store);
    const exe = new Map(all.map((j) => [j.name, j.argv[0]!]));
    // A binary this run did not build is judged by the next run that builds it.
    wide = findings(scoped.map((n) => ({ name: n, owner: ownerOf(n), exe: exe.get(n) ?? "", reads: reads.get(n), known: r.graph!.has(ownerOf(n)) })));
  }

  const times: Record<string, number> = { ...last };
  for (const [n, res] of results) times[n] = Math.round(res[0] * 100) / 100;
  writeQuiet(TEST_TIMES, JSON.stringify(sorted(times), null, 1));

  // Passing binaries first, by name, so the tail a red step prints is the failures.
  const names = new Set(failed.map((j) => j.name));
  const out: string[] = note ? [note] : [];
  const held = scoped.filter((n) => !results.has(n)).sort();
  if (held.length) out.push(`${held.length} of ${scoped.length} test binaries not re-run: the change reaches nothing each one ran`);
  for (const n of held) out.push(`     Unchanged ${n}\n${testGreen(r.store, n)?.result ?? ""}`);
  for (const n of [...results.keys()].sort()) if (!names.has(n)) out.push(`     Running ${n}\n${results.get(n)![2]}`);
  for (const j of failed) out.push(`     Running ${j.name}  -- FAILED, exit ${results.get(j.name)![1]}\n${results.get(j.name)![2]}`);
  for (const j of failed) out.push(alone.get(j.name) ? RERUN_ALONE_PASSED(j.name) : `error: \`${j.name}\` failed, alone as well; \`${j.rerun}\` runs it again.`);
  for (const line of wide) out.push(`error: ${line}`);
  return [failed.length || wide.length ? 1 : 0, out.join("\n")];
}

// ---- the case trees --------------------------------------------------------------------------------

/** A case tree's step: the selected cases, recorded. */
async function runCases(r: Run, s: Step): Promise<[number, string]> {
  const cases = casesToRun(r, s.name);
  const total = caseFiles().filter((p) => p.startsWith(`tests/${s.name}/`)).length;
  const jobs = cpus().length || 4;
  const got = await recordCases(r.rec, covwsNvs(), cases, { jobs, batch: 128, onBatch: (done, all) => stepDetail(s.name, `${done}/${all} cases`) });
  for (const p of cases) r.ran.add(caseId(p));
  const red = [...got.verdicts.values()].filter((v) => v === "red").length;
  const head = `${cases.length} of ${total} case(s) of tests/${s.name} selected`;
  return [red > 0 || got.failed > 0 ? 1 : 0, `${head}\n${got.out}\n${got.passed} passed, ${got.failed} failed, ${got.skipped} skipped\n`];
}

// ---- the tools' own gate ---------------------------------------------------------------------------

/** The `bun test` files this run runs, repo-relative. */
function nvTestsToRun(r: Run): string[] {
  const files = nvTestFiles();
  return r.opts.noCache ? files : files.filter((f) => r.sel.selected.has(nvTestId(f)));
}

/**
 * The `nv` step: `tsc` when the change reaches what it reads, then each selected `bun test` file in a
 * process of its own, a few at a time, each recorded as its own atom (`select/nvtests.ts`). A red `tsc`
 * stops the step before any test runs.
 */
async function runSelftest(r: Run, s: Step): Promise<[number, string]> {
  const out: string[] = [];
  if (picked(r, NV_TSC)) {
    stepDetail(s.name, "tsc");
    const t = await runTsc(r.rec);
    r.ran.add(NV_TSC);
    if (t.verdict === "red") return [1, `${t.out}\nnv selftest: tsc failed with exit ${t.code}`];
    out.push("nv selftest: the types check");
  } else out.push("nv selftest: the types check (the change reaches nothing tsc reads)");
  const files = nvTestsToRun(r);
  let passed = 0;
  let failed = 0;
  let done = 0;
  const reds: string[] = [];
  const width = Math.max(1, Math.min(4, Math.floor((cpus().length || 4) / 4)));
  await pool(files, width, async (file) => {
    const t = await runNvTest(r.rec, file);
    passed += t.passed;
    failed += t.failed;
    if (t.verdict === "red") reds.push(`-- ${file}: exit ${t.code}\n${t.out}`);
    r.ran.add(nvTestId(file));
    stepDetail(s.name, `${++done}/${files.length} test files`);
  });
  const all = nvTestFiles().length;
  out.push(`${files.length} of ${all} test file(s) run`);
  out.push(...reds);
  out.push(` ${passed} pass`, ` ${failed} fail`);
  if (reds.length > 0) {
    out.push(`nv selftest: bun test failed in ${reds.length} file(s)`);
    return [1, out.join("\n")];
  }
  out.push("nv selftest: every test passes");
  return [0, out.join("\n")];
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
const FORMATTED = "verify:nvs-fmt";

/** Each changed `.nvs` file's digest as `nvs-fmt` last left it, from the store. */
function formattedOf(store: SelectStore): Record<string, string> {
  try {
    const got = JSON.parse(store.verdict(FORMATTED)?.verdict ?? "{}");
    return got && typeof got === "object" && !Array.isArray(got) ? got : {};
  } catch {
    return {};
  }
}

/** `[todo, changed]`: the changed `.nvs` files the formatter has not been over as they now stand, and
 * all of the changed ones, which are the only paths the record still has a reason to hold. */
async function unformatted(store: SelectStore): Promise<[string[], string[]]> {
  const changed = await changedSources();
  const formatted = formattedOf(store);
  const todo: string[] = [];
  for (const rel of changed) {
    try {
      if (formatted[rel] !== digestOf(rel)) todo.push(rel);
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
    const all = [...out.matchAll(CASES_RE)];
    const m = all.at(-1);
    if (!m) return "ran, but printed no `N passed` line -- check the log";
    const of = /^(\d+) of (\d+) case\(s\)/m.exec(out);
    return `${m[1]} passed, ${m[2]} failed` + (Number(m[3]) ? `, ${m[3]} skipped` : "") + (of && of[1] !== of[2] ? `  (${of[1]} of ${of[2]} selected)` : "");
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
    if (m) return `${m[1]} recorded gap(s), each owned by a milestone still ahead or the running goal`;
    m = /(\d+) recorded gap\(s\) name an owner this gate refuses/.exec(out);
    return m ? `${m[1]} recorded gap(s) name an owner the gate refuses` : NO_SUMMARY;
  },
  nv(out) {
    if (out.includes("nv selftest: every test passes")) {
      const m = /^\s*(\d+) pass$/m.exec(out);
      const of = /^(\d+) of (\d+) test file\(s\) run/m.exec(out);
      const files = of && of[1] !== of[2] ? `  (${of[1]} of ${of[2]} files run)` : "";
      return (m ? `the types check, ${m[1]} test(s) pass` : "the types check, every test passes") + files;
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

/** A `bun nv` step: its reads are recorded through `NV_READS_LOG`, and whatever it runs of `nvs` by
 * coverage. `extra` adds the files it is given beside what it was seen to read. */
function bunStep(r: () => Run, name: string, args: string[], extra: (keys: Keyed) => void = () => {}, env: () => Record<string, string> = () => ({})): Step {
  const rec = recordName(`step:${name}`);
  const log = () => join(r().rec.dir, `${rec}.reads`);
  return step(name, args, summaries[name]!, {
    exe: "bun",
    atom: `step:${name}`,
    runner: async (s) => {
      rmSync(log(), { force: true });
      return spawnOut([s.exe, ...s.args], s.cwd, { ...r().rec.env(rec), NV_READS_LOG: log(), ...env() }, undefined, true);
    },
    footprint: async () => {
      const keys = nvKeys(log());
      rmSync(log(), { force: true });
      for (const [k, d] of (await r().rec.keysOf(rec, [covwsNvs()]))?.keys ?? []) keys.set(k, d);
      extra(keys);
      return keys;
    },
  });
}

/** The rustdoc gate. `private_intra_doc_links` is allowed: a link to a crate-private item in a crate
 * nobody publishes is a correct reference rustdoc will not turn into an anchor. Always the whole
 * workspace, whatever `-p` says. */
function docStep(r: () => Run): Step {
  const { env, args } = covwsCargo();
  return step("doc", ["doc", "--no-deps", "--workspace", ...args], summaries.doc!, {
    env: { ...env, RUSTDOCFLAGS: "-A rustdoc::private_intra_doc_links -D warnings" },
    atom: "step:doc",
    footprint: async () => buildKeys(r().graph),
  });
}

/** `built` says whether the `covws` build's `nvs` is on disk; `stepNames` passes it so its answer does not
 * depend on the tree it runs in. */
function stepsFor(opts: Opts, r: () => Run, built = existsSync(covwsNvs())): Step[] {
  if (opts.doc) return [docStep(r)];
  const { env: covEnv, args: target } = covwsCargo();
  const nvs = covwsNvs();
  const steps: Step[] = [];
  let selftest: Step | null = null;
  if (!opts.fast) {
    // `-l` names each file rustfmt rewrote, which the summary quotes.
    steps.push(step("fmt", ["fmt", "--all", "--", "-l"], summaries.fmt!, { atom: "step:fmt", footprint: fmtKeys }));
    // The script steps decide what the tree means rather than whether it builds, and read manifests,
    // doc comments or the tools, so `-p` narrows none of them.
    steps.push(bunStep(r, "lints", ["nv", "lints", "--check"]));
    steps.push(bunStep(r, "directives", ["nv", "directives", "--check"]));
    steps.push(bunStep(r, "template", ["nv", "directives", "--check-template"]));
    steps.push(bunStep(r, "owners", ["nv", "owners", "--check"]));
    if (existsSync(join(ROOT, "package.json"))) {
      // `tsc` over every module under `tools/nv`, then each selected `bun test` file on its own. Some of
      // those files run the `covws` `nvs`, so in a tree that has never built it the step runs after
      // `build` rather than beside it.
      selftest = step("nv", ["nv", "selftest"], summaries.nv!, { exe: "bun", runner: (s) => runSelftest(r(), s) });
      if (built) steps.push(selftest);
    }
    if (existsSync(join(FUZZ, "Cargo.toml"))) {
      steps.push(
        step("fuzz-lock", ["metadata", "--format-version", "1", "--offline", "--manifest-path", "fuzz/Cargo.toml"], summaries["fuzz-lock"]!, {
          runner: runFuzzLock,
          atom: "step:fuzz-lock",
          footprint: async () => {
            const keys: Keyed = new Map();
            maybe(keys, "fuzz/Cargo.toml");
            maybe(keys, "fuzz/Cargo.lock");
            return keys;
          },
        }),
      );
    }
  }
  // Bare, whatever `-p` says: a `-p` build writes a second copy of every workspace crate.
  steps.push(step("build", ["build", ...target], summaries.build!, { env: covEnv, atom: "step:build", footprint: async () => buildKeys(r().graph) }));
  if (selftest !== null && !built) steps.push(selftest);
  // Whole-workspace runs only from here on for the steps that run `nvs`: a scoped build leaves no
  // binary the tree can trust.
  if (!opts.fast && !opts.package) {
    steps.push(step("nvs-fmt", ["fmt", "<each new or modified .nvs under tests/, examples/>"], summaries["nvs-fmt"]!, { exe: nvs, runner: runNvsFmt }));
  }
  steps.push(step("test", ["test", ...target], summaries.test!, { runner: (s) => runTests(r(), s) }));
  if (!opts.fast) {
    if (!opts.package) {
      for (const tree of CASE_TREES) {
        if (existsSync(join(ROOT, "tests", tree))) steps.push(step(tree, ["test", "--cases", "<the selected cases>", `tests/${tree}`], summaries.cases!, { exe: nvs, runner: (s) => runCases(r(), s) }));
      }
      steps.push(bunStep(r, "reference", ["nv", "reference"], () => {}, () => ({ NVS_BIN: nvs })));
    }
    steps.push(
      step("clippy", ["clippy", "--all-targets", ...target, "--", "-D", "warnings"], summaries.clippy!, {
        env: { CARGO_TARGET_DIR: covEnv.CARGO_TARGET_DIR! },
        atom: "step:clippy",
        footprint: async () => {
          const keys = buildKeys(r().graph);
          maybe(keys, "clippy.toml");
          maybe(keys, ".clippy.toml");
          return keys;
        },
      }),
    );
    // Last, because it is the one step that is not `cargo` or a script.
    if (!opts.package && existsSync(join(EXTENSION, "package.json"))) {
      const rec = recordName("step:extension");
      steps.push(
        step("extension", ["run", "--silent", "test:headless"], summaries.extension!, {
          exe: process.platform === "win32" ? "npm.cmd" : "npm",
          cwd: EXTENSION,
          atom: "step:extension",
          runner: (s) => spawnOut([s.exe, ...s.args], s.cwd, { ...r().rec.env(rec), NVS_BIN: nvs }, undefined, true),
          footprint: async () => {
            const keys: Keyed = new Map([["tree:editors/vscode", ""]]);
            for (const [k, d] of (await r().rec.keysOf(rec, [nvs]))?.keys ?? []) keys.set(k, d);
            return keys;
          },
        }),
      );
    }
  }
  return steps;
}

/** One step's command as a reader types it. */
function shown(s: Step): string {
  let line = [basename(s.exe), ...s.args].join(" ");
  const env = Object.entries(s.env ?? {}).filter(([k]) => k !== "RUSTC_WORKSPACE_WRAPPER" && k !== "CARGO_TARGET_DIR");
  if (env.length) line = env.map(([k, v]) => `${k}=${v}`).join(" ") + " " + line;
  if (s.cwd !== ROOT) line += `   (in ${relative(ROOT, s.cwd).replace(/\\/g, "/")})`;
  return line;
}

const noRun = (): Run => {
  throw new Error("verify: a step asked for the run before it began");
};

/** `--list`: the order the gate walks, narrowed by `-p`, `--fast` and `--doc` exactly as a run is. */
function listSteps(opts: Opts): number {
  const steps = stepsFor(opts, noRun);
  const scope = opts.package ? ` (-p ${opts.package})` : "";
  console.log(`verify: ${steps.length} step(s) in this order${scope}, each run only when the change reaches it; \`--list\` runs none of them.`);
  steps.forEach((s, i) => console.log(`  ${i + 1}. ${s.name.padEnd(13)} ${shown(s)}`));
  return 0;
}

// ---- output ----------------------------------------------------------------------------------------

function clock(seconds: number): string {
  if (seconds >= 60) return `${Math.floor(seconds / 60)}m${String(Math.floor(seconds) % 60).padStart(2, "0")}s`;
  return `${seconds.toFixed(0)}s`;
}

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

/** One line: what the change is and how much of each kind it selects. */
function selectionLine(r: Run): string {
  if (r.opts.noCache) return "verify: --no-cache, so every step, test binary and case runs";
  const c = r.change;
  const kinds: Record<string, number> = {};
  for (const s of r.sel.selected.values()) kinds[s.kind] = (kinds[s.kind] ?? 0) + 1;
  const what = Object.entries(kinds)
    .map(([k, n]) => `${n} ${k}`)
    .join(", ");
  const since = c.full ? c.global : `${c.changes.length} path(s) changed since the recorded tree (${c.since.slice(0, 12)})`;
  return `verify: ${since}; the store selects ${what || "nothing"}`;
}

// ---- --start and --wait ----------------------------------------------------------------------------

/** Starts the same verification as an `nv bg` job and returns at once, so the wrap is written while it
 * runs. It is the same steps, the same store and the same exit status. */
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

const lastSlot = (name: string) => `verify:${name}`;

async function verify(opts: Opts): Promise<number> {
  await hooksNote();
  const store = new SelectStore();
  const graph = await metadata();
  const first = await select(store, graph);
  const rec = await Recorder.open(store, first.change.view, graph, "verify", { say: (l) => console.error(l) });
  const r: Run = { opts, store, graph, ...first, rec, ran: new Set() };
  try {
    return await walk(r);
  } finally {
    rec.close();
    store.close();
  }
}

async function walk(r: Run): Promise<number> {
  const { opts, store } = r;
  const steps = stepsFor(opts, () => r);
  const scope = opts.package ? ` (-p ${opts.package})` : "";
  console.log(selectionLine(r));
  for (const s of steps) if (s.name === "nvs-fmt") [s.todo, s.changed] = await unformatted(store);

  const done: Step[] = [];
  /** The steps not run, with the summary of their last green run. */
  const unchanged = new Map<string, string>();
  let failed: Step | null = null;
  // `fmt` red: reported only if nothing after it is.
  let deferred: Step | null = null;
  const began = performance.now();

  /** Whether `s` runs; a step that does not is noted with its last summary. */
  const needed = (i: number, s: Step): boolean => {
    let run = wanted(r, s);
    // `build` also runs for any later step that uses what it leaves on disk.
    if (!run && s.name === "build") run = steps.slice(i + 1).some((t) => usesBinary(t.name) && wanted(r, t));
    if (!run) unchanged.set(s.name, s.name === "nvs-fmt" ? "nothing new to format" : (store.verdict(lastSlot(s.name))?.verdict ?? "the change reaches nothing it reads"));
    else unchanged.delete(s.name);
    return run;
  };

  const prepare = (i: number, s: Step) => progress(done, s, steps.length, undefined, i + 1);

  /** Records a finished step's own atom. */
  const recordStep = async (s: Step, ok: boolean) => {
    if (!s.atom || !s.footprint) return;
    let keys: Keyed;
    try {
      keys = await s.footprint(s);
    } catch {
      keys = new Map([[WILD, ""]]);
    }
    store.recordRun(s.atom, { def: digest(cmdOf(s)), verdict: ok ? "green" : "red", keys });
    r.ran.add(s.atom);
  };

  /** One finished run's verdict, taken in list order; false when it stops the run. */
  const settle = async (s: Step, ok: boolean): Promise<boolean> => {
    await recordStep(s, ok);
    if (s.formatted && Object.keys(s.formatted).length) {
      // A path that is no longer new or modified has been committed, and is dropped.
      const kept = new Set(s.changed ?? []);
      const was = formattedOf(store);
      store.putVerdict(FORMATTED, "", JSON.stringify({ ...Object.fromEntries(Object.entries(was).filter(([k]) => kept.has(k))), ...s.formatted }));
    }
    if (WRITES.has(s.name) && /^rewrote |\.rs\s*$/m.test(s.out)) {
      // It rewrote files, so the change is read again, and what ran over the old text and is selected
      // again counts as not run.
      Object.assign(r, await select(store, r.graph));
      for (const id of [...r.ran]) if (r.sel.selected.has(id) && id !== s.atom) r.ran.delete(id);
      r.rec.remap(r.change.view, r.graph);
    }
    if (s.name === "fmt" && !ok) {
      deferred = s;
      return true;
    }
    if (!ok) {
      failed = s;
      return false;
    }
    store.putVerdict(lastSlot(s.name), "", s.summarize(s.out));
    done.push(s);
    return true;
  };

  /** The script steps from `i` on, run at the same time as the `build` after them. */
  const besideBuild = async (i: number): Promise<number> => {
    let j = i;
    while (j < steps.length && BESIDE_BUILD.has(steps[j]!.name)) j++;
    const group = j < steps.length && steps[j]!.name === "build" ? steps.slice(i, j + 1) : steps.slice(i, j);
    const todo = group.map((s, k) => [i + k, s] as const).filter(([k, s]) => needed(k, s));
    const inFlight = todo.map(([k, s]) => {
      prepare(k, s);
      return [s, runStep(s)] as const;
    });
    for (const [s, p] of inFlight) if (!(await settle(s, await p))) break;
    await Promise.allSettled(inFlight.map(([, p]) => p));
    return group.length;
  };

  /** `test`, and the steps after it, started on one lane while its slowest binaries still run. */
  const withTail = async (i: number): Promise<number> => {
    const test = steps[i]!;
    const testRuns = needed(i, test);
    const after = steps
      .slice(i + 1)
      .map((s, k) => [i + 1 + k, s] as const)
      .filter(([k, s]) => needed(k, s));
    let release!: () => void;
    const tailReached = new Promise<void>((res) => (release = res));
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
    let ok = true;
    if (testRuns) {
      prepare(i, test);
      ok = await runStep(test);
    }
    if (!ok) stop = true;
    release();
    await lane;
    if (!testRuns || (await settle(test, ok))) {
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
      if (needed(i, s)) {
        prepare(i, s);
        await settle(s, await runStep(s));
      }
      i++;
    }
  }
  const stopped = failed as Step | null;
  // A step after the one that stopped the run was not reached, whatever it would have been.
  if (stopped !== null) for (const s of steps.slice(steps.indexOf(stopped) + 1)) unchanged.delete(s.name);
  const red: Step | null = stopped ?? (deferred as Step | null);
  progress(done, undefined, 0, red ? 1 : 0);

  // Red or green, the tree moves: what ran is recorded with its verdict, a red atom stays selected as
  // red, and what the change reached that did not run is owed.
  let owedNote = "";
  const { owed } = advance(store, r.change, r.sel, r.ran, r.graph);
  if (owed > 0) owedNote = `; ${owed} reached atom(s) this run did not run stay owed (\`bun nv select\` names them)`;

  const total = (performance.now() - began) / 1000;
  const lines = () => {
    for (const s of steps) {
      const e = unchanged.get(s.name);
      if (e !== undefined) console.log(`  ${s.name.padEnd(13)} ${"--".padStart(6)}   ${e}`);
      else if (done.includes(s)) console.log(`  ${s.name.padEnd(13)} ${clock(s.seconds).padStart(6)}   ${s.summarize(s.out)}`);
    }
  };
  const ranLine = () => {
    const kinds: Record<string, number> = {};
    for (const id of r.ran) kinds[id.slice(0, id.indexOf(":"))] = (kinds[id.slice(0, id.indexOf(":"))] ?? 0) + 1;
    return Object.entries(kinds)
      .map(([k, n]) => `${n} ${k}`)
      .join(", ");
  };

  if (red === null && done.length === 0) {
    console.log(`verify: green, the change reaches nothing a step reads -- nothing to re-run${scope}${owedNote}`);
    lines();
    console.log("\n`--no-cache` runs every step anyway.");
    return 0;
  }

  const note = unchanged.size ? ` -- ${unchanged.size} not reached (\`--\` below)` : "";
  if (red === null) {
    console.log(`verify: ${done.length + unchanged.size} of ${steps.length} green in ${clock(total)}${scope}${note}; ran ${ranLine() || "nothing recorded"}${owedNote}`);
    lines();
    return 0;
  }

  store.putVerdict(lastSlot(red.name), "", "");
  console.log(`verify: FAILED at ${red.name} (step ${steps.indexOf(red) + 1} of ${steps.length}) after ${clock(total)}${scope}${note}${owedNote}`);
  lines();
  console.log(`  ${red.name.padEnd(13)} ${"---".padStart(6)}   exit ${red.code}`);
  const [body, hidden] = tail(red.out, opts.full ? 0 : TAIL_LINES);
  console.log(`\n-- \`${cmdOf(red)}\`` + (hidden ? `, last ${TAIL_LINES} lines` : ""));
  console.log(body);
  if (hidden) console.log(`\n... ${hidden} earlier line(s) hidden`);
  console.log(`\nfull output: .agent-tmp/verify-${red.name}.log`);
  return 1;
}

/** The steps an unnarrowed run walks, in order. */
export function stepNames(built = true): string[] {
  return stepCommands(built).map(([name]) => name);
}

/** Each step an unnarrowed run walks, as its name and the command it starts. */
export function stepCommands(built = true): [string, string][] {
  return stepsFor({ fast: false, doc: false, full: false, noCache: false, start: false, wait: false, list: false }, noRun, built).map((s) => [s.name, cmdOf(s)]);
}

/** What an unnarrowed `bun nv verify` would run over the tree as it stands: each step it would start,
 * and the test binaries and cases, from the store's selection. `nv affected` prints it. */
export async function verifyPlan(graph: Graph | null): Promise<{ steps: string[]; tests: string[]; cases: Record<string, string[]>; change: ChangeSet; sel: Selection }> {
  const store = new SelectStore();
  try {
    const got = await select(store, graph);
    const opts: Opts = { fast: false, doc: false, full: false, noCache: false, start: false, wait: false, list: false };
    const r = { opts, store, graph, ...got, rec: null as unknown as Recorder, ran: new Set<string>() };
    const steps = stepsFor(opts, () => r);
    for (const s of steps) if (s.name === "nvs-fmt") [s.todo, s.changed] = await unformatted(store);
    const run = steps.filter((s) => wanted(r, s)).map((s) => s.name);
    const after = steps.slice(steps.findIndex((s) => s.name === "build") + 1);
    if (!run.includes("build") && after.some((s) => usesBinary(s.name) && run.includes(s.name))) run.push("build");
    const order = steps.map((s) => s.name).filter((n) => run.includes(n));
    return { steps: order, tests: testsToRun(r), cases: Object.fromEntries(CASE_TREES.map((t) => [t, casesToRun(r, t)])), change: got.change, sel: got.sel };
  } finally {
    store.close();
  }
}


const USAGE = ["usage: nv verify [-h] [-p PACKAGE] [--fast] [--doc] [--full] [--no-cache]", "                 [--start] [--wait] [--list]"].join("\n");

export async function run(args: string[]): Promise<number> {
  let parsed;
  try {
    parsed = parseArgs(args, { flags: ["--fast", "--doc", "--full", "--no-cache", "--start", "--wait", "--list"], valued: ["--package"], short: { "-p": "--package" } });
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
