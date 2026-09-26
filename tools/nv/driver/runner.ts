// The acceptance sweep on the selection store (`tools/nv/select/`). `PlanSweep.open` reads the change
// since the store's tree once, asks the store which atoms it reaches, and groups the plan's checks into
// atoms (`select/checks.ts`). `reached` says whether a check has a picked atom, and `check` runs only the
// picked atoms of a reached check, records every one, and judges the check. `close` moves the store's
// tree past the change, green or red: an atom that ran records its footprint and verdict, and an atom
// the change reached that did not run is owed (`record.ts` `advance`).
//
// Every run records, whatever it is:
//
// - a fixture or an `{nvs}` command runs the covws `nvs` (`lib/covws.ts`) with its coverage and its
//   footprint log going to the check's own atom, and the compile cache off;
// - a case runs through `nvs test --cases` (`recordCases`), shared with `nv verify`'s case trees;
// - a test binary runs whole in its package's directory, shared with `nv verify`'s `test` step, which
//   also keeps its last green lines for a check that names a test;
// - a `bun nv` command runs with `NV_READS_LOG`, so its reads and the modules it loaded are its footprint,
//   and with `NV_SELECT_NO_ADVANCE`, so a `bun nv proofs` it starts records its programs and leaves the
//   tree to the sweep;
// - the tools' `tsc` and each of their test files run as `select/nvtests.ts` runs them for `nv verify`;
// - a heavy check runs as it always has, with the footprint logs of what it starts, and holds
//   `heavyKeys` besides.
//
// A sweep shares every process between the checks that ask for the same one. Two checks naming one
// `argv` in one directory get one run, recorded under each check's atom, and each judges its own exit
// and `want` against it; the same holds for a case, a test binary and a tools test file. The
// `nv proofs --verify --group` checks of one tier share a run too: the first one reached starts one
// `nv proofs --verify` over every group the tier's reached checks name, and each check is judged on its
// own groups' sections of what that run prints.
//
// One covws build serves every fixture, suite and `{nvs}` command, and one `cargo test --no-run` every
// test binary. A check that measures the release CLI gets `cargo build --release -p nvs-cli` first,
// since nothing else builds it.

import { cpus } from "node:os";
import { join } from "node:path";
import { metadata, type Graph } from "../keys/graph.ts";
import { covwsCargo, covwsNvs } from "../lib/covws.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { linked, releaseCli } from "../lib/relink.ts";
import { recordName } from "../proofs/run.ts";
import { caseFiles, caseId, fileDef, nvTestFiles, nvTestId, proofDef } from "../select/atoms.ts";
import { commandKeys, grouped, type Grouped, groupDirsOf, heavyKeys, LEG_TREES, LEGS, legId, onDisk, type PlanContext } from "../select/checks.ts";
import { NO_ADVANCE_ENV, advance, caseSkipped, fullChange, pool, putTestGreen, Recorder, recordCases, testGreen } from "../select/record.ts";
import { NV_TSC, runNvTest, runTsc, type ToolRun } from "../select/nvtests.ts";
import { type ChangeSet, computeChange, query, rustFiles, type Selection } from "../select/select.ts";
import { nvKeys, testKeys } from "../select/seed.ts";
import { type Keyed, SelectStore, type Verdict as AtomVerdict } from "../select/store.ts";
import { ENV as READS_ENV } from "../lib/reads.ts";
import {
  type Check,
  firstErrLine,
  judgeCommand,
  judgeProgram,
  judgeTests,
  measuresReleaseCli,
  type Outcome,
  PROGRAM_KINDS,
  proofGroups,
  proofOutcome,
  type TestExe,
  testExecutables,
  type Verdict,
} from "./accept.ts";

const TIMEOUT_MS = 1800 * 1000;

/** Runs `argv`, and turns a program that cannot start into exit -1 with the reason on stderr. Whatever
 * it started and left behind is killed with it. */
async function capture(argv: string[], cwd: string, env?: Record<string, string>): Promise<Outcome> {
  try {
    const r = await run(argv, { cwd, timeoutMs: TIMEOUT_MS, reap: true, ...(env ? { env } : {}) });
    return { code: r.timedOut ? -1 : r.code, out: r.stdout, err: r.timedOut ? `timed out after ${TIMEOUT_MS / 1000}s` : r.stderr };
  } catch (e) {
    return { code: -1, out: "", err: String((e as Error).message ?? e) };
  }
}

const none = (fail: string): Verdict => ({ fail, short: "" });
const green: Verdict = { fail: "", short: "" };

export interface OpenOptions {
  /** Pick every atom of every check, whatever changed. */
  full: boolean;
  /** Called with each process before it starts. */
  onRun?: (what: string) => void;
  /** A line about what the sweep could not read, which never stops it. */
  say?: (line: string) => void;
}

/** One sweep over a plan: the change it reads, what that reaches, and the runs it makes. */
export class PlanSweep {
  /** Every atom this sweep ran and recorded. */
  readonly ran = new Set<string>();
  private readonly started = Date.now();
  private cli: Promise<string | { fail: string }> | undefined;
  private exes: Promise<Map<string, TestExe[]> | { fail: string }> | undefined;
  private release: Promise<Outcome> | undefined;
  private files: Promise<string[]> | undefined;
  private readonly shared = new Map<string, Promise<{ o: Outcome; keys: Keyed }>>();
  private readonly exeRuns = new Map<string, Promise<{ o: Outcome; verdict: AtomVerdict }>>();
  private readonly toolRuns = new Map<string, Promise<ToolRun>>();
  private readonly casesDone = new Set<string>();
  private readonly exeShown = new Map<string, string>();
  private caseOut = "";
  private proofs: { groups: string[]; run?: Promise<{ o: Outcome; keys: Keyed }> } | undefined;
  private runs = 0;

  private constructor(
    readonly store: SelectStore,
    readonly graph: Graph | null,
    readonly change: ChangeSet,
    readonly sel: Selection,
    readonly groups: Map<string, Grouped>,
    readonly plan: Check[],
    private readonly rec: Recorder,
    private readonly label: (n: number) => string,
    private readonly o: OpenOptions,
  ) {}

  /** Reads the change since the store's tree and what it reaches of `plan`, the whole plan's checks. A
   * store with no tree, or one git can no longer name, reaches everything. */
  static async open(plan: Check[], label: (n: number) => string, o: OpenOptions): Promise<PlanSweep> {
    const store = new SelectStore();
    const graph = await metadata();
    let change: ChangeSet;
    try {
      change = store.base() === null ? await fullChange() : await computeChange(store, { graph });
    } catch (e) {
      change = await fullChange(ROOT, `the recorded tree could not be read: ${(e as Error).message.split("\n")[0]}`);
    }
    const cases = caseFiles();
    const nvTests = nvTestFiles();
    const ctx: PlanContext = { graph, cases, nvTests, groupDirs: groupDirsOf(store), proofs: store.atoms("proof").map((a) => a.id.slice(6)) };
    const groups = new Map(plan.map((c) => [c.id, grouped(c, ctx)]));
    const discovered = new Map<string, string>();
    const defs = new Map<string, string>();
    for (const g of groups.values()) {
      for (const a of g.atoms) {
        if (discovered.has(a)) continue;
        if (a.startsWith("case:")) discovered.set(a, fileDef(a.slice(5)));
        else if (a.startsWith("proof:")) discovered.set(a, proofDef(a.slice(6)));
        else if (a.startsWith("nvtest:")) discovered.set(a, fileDef(a.slice(7)));
        else if (a === NV_TSC || a.startsWith("test:")) discovered.set(a, "");
      }
      if (g.own) {
        discovered.set(g.own.id, g.own.def);
        defs.set(g.own.id, g.own.def);
      }
    }
    for (const leg of LEGS) {
      discovered.set(legId(leg), LEG_DEF);
      defs.set(legId(leg), LEG_DEF);
    }
    const sel = query(store, change, { discovered: [...discovered].map(([id, def]) => ({ id, def })), defs });
    const rec = await Recorder.open(store, change.view, graph, "sweep", { say: o.say ?? (() => {}) });
    return new PlanSweep(store, graph, change, sel, groups, plan, rec, label, o);
  }

  private picked(atom: string): boolean {
    return this.o.full || this.sel.selected.has(atom);
  }

  private say(what: string): void {
    this.o.onRun?.(what);
  }

  private labelOf(c: Check): string {
    return `${c.name ?? c.file ?? c.id} [${this.label(c.stage)}]`;
  }

  /** What a check needs run beyond its picked atoms, for what the plan asks of it: each binary of a check
   * that names a test with no green run kept, and each named case no run has said is skipped or not. */
  private lacking(c: Check, g: Grouped): string[] {
    if (g.how === "tests" && (c.tests ?? []).length > 0) return (g.tests ?? []).filter((n) => testGreen(this.store, n) === null).map((n) => `test:${n}`);
    if (g.how === "cases") return (c.cases ?? []).filter((p) => onDisk(p) && caseSkipped(this.store, p) === null).map(caseId);
    return [];
  }

  /** Whether the change reaches `c`: one of its atoms is picked, it is never memoized, a group of it has
   * never run, or what the plan asks of it cannot be judged from the store. */
  reached(c: Check): boolean {
    if (this.o.full || c.memoize === false) return true;
    const g = this.groups.get(c.id);
    if (g === undefined || g.unknown) return true;
    return g.atoms.some((a) => this.sel.selected.has(a)) || this.lacking(c, g).length > 0;
  }

  /** A check the change does not reach, judged from the store: its atoms are green, and a suite's named
   * cases and `minPassing` and a test check's named tests are read off their last runs. */
  judged(c: Check): Verdict {
    const g = this.groups.get(c.id);
    if (g?.how === "cases") return this.judgeSuite(c, g);
    if (g?.how === "tests") return this.judgeTestCheck(c, g);
    return green;
  }

  /** Counts for the sweep's summary: the plan's checks the change reaches, and the atoms it picked. */
  counts(checks: Check[]): { reached: number; of: number; picked: Record<string, number> } {
    const picked: Record<string, number> = {};
    for (const s of this.sel.selected.values()) picked[s.kind] = (picked[s.kind] ?? 0) + 1;
    return { reached: checks.filter((c) => this.reached(c)).length, of: checks.length, picked };
  }

  // ---- what the sweep builds ---------------------------------------------------------------------

  /** The covws `nvs`, built once by the workspace build `nv verify` also runs. */
  private binary(): Promise<string | { fail: string }> {
    this.cli ??= (async () => {
      this.say("cargo build (covws)");
      const { env, args } = covwsCargo();
      const r = await capture(["cargo", "build", "--quiet", ...args], ROOT, env);
      if (r.code !== 0) return { fail: `the native build failed -- ${firstErrLine(r)}` };
      return covwsNvs();
    })();
    return this.cli;
  }

  private testExes(): Promise<Map<string, TestExe[]> | { fail: string }> {
    this.exes ??= (async () => {
      this.say("cargo test --no-run (covws)");
      const { env, args } = covwsCargo();
      const r = await capture(["cargo", "test", "--no-run", ...args, "--message-format=json"], ROOT, env);
      if (r.code !== 0) return { fail: `the workspace test build failed -- ${firstErrLine(r)}` };
      return testExecutables(r.out);
    })();
    return this.exes;
  }

  private releaseCli(): Promise<Outcome> {
    this.release ??= (async () => {
      this.say("cargo build --release -p nvs-cli");
      // An editor running `nvs lsp` out of this tree holds the binary the link replaces;
      // `tools/nv/lib/relink.ts` moves it aside so the retry lands.
      return linked(releaseCli(), () => capture(["cargo", "build", "--release", "-p", "nvs-cli"], ROOT), (r) => r.err);
    })();
    return this.release;
  }

  private rustFiles(): Promise<string[]> {
    this.files ??= rustFiles();
    return this.files;
  }

  // ---- recorded runs -----------------------------------------------------------------------------

  /** One recorded run of `argv` in `cwd`, shared by every check that asks for the same one: what it
   * printed, and what it was seen to use. A `bun nv` process's reads are recorded besides. */
  private recorded(argv: string[], cwd: string, what: string, extra: Record<string, string> = {}): Promise<{ o: Outcome; keys: Keyed }> {
    const key = `${cwd}\0${argv.join("\0")}`;
    let p = this.shared.get(key);
    if (p === undefined) {
      p = (async () => {
        this.say(what);
        const name = `sw${++this.runs}`;
        const nv = argv[0] === "bun" && argv[1] === "nv";
        const log = join(this.rec.dir, `${name}.reads`);
        const env: Record<string, string> = { ...this.rec.env(name), NO_COLOR: "1", ...extra };
        if (nv) Object.assign(env, { [READS_ENV]: log, [NO_ADVANCE_ENV]: "1" });
        const o = await capture(argv, join(ROOT, cwd), env);
        const keys: Keyed = nv ? nvKeys(log) : new Map();
        for (const [k, d] of (await this.rec.keysOf(name, [covwsNvs()]))?.keys ?? []) keys.set(k, d);
        return { o, keys };
      })();
      this.shared.set(key, p);
    }
    return p;
  }

  /** Records one run of a check's own atom. */
  private recordOwn(g: Grouped, verdict: AtomVerdict, keys: Keyed): void {
    if (!g.own) return;
    this.store.recordRun(g.own.id, { def: g.own.def, verdict, keys });
    this.ran.add(g.own.id);
  }

  /** The `argv` a check runs, with `{nvs}` as the covws binary, or the build's failure: a command's own,
   * `nvs <args>` for a suite, `cargo <args>` for any other. */
  private async argvOf(c: Check): Promise<string[] | { fail: string }> {
    const argv = c.kind === "command" ? (c.argv ?? []) : c.kind === "nvs-suite" ? ["{nvs}", ...(c.args ?? [])] : ["cargo", ...(c.args ?? [])];
    if (!argv.includes("{nvs}")) return argv;
    const exe = await this.binary();
    if (typeof exe !== "string") return exe;
    return argv.map((a) => (a === "{nvs}" ? exe : a));
  }

  // ---- the checks --------------------------------------------------------------------------------

  /** Names the checks the sweep is about to reach, so the first `nv proofs --verify --group` check among
   * them runs every group they name in one process. Fewer than two groups is no batch. */
  batch(checks: Check[]): void {
    const groups = [...new Set(checks.flatMap((c) => proofGroups(c) ?? []))];
    this.proofs = groups.length > 1 ? { groups } : undefined;
  }

  /** Runs the picked atoms of `c`, records each, and judges it. */
  async check(c: Check): Promise<Verdict> {
    const g = this.groups.get(c.id) ?? grouped(c, { graph: this.graph, cases: [], nvTests: [], groupDirs: new Map(), proofs: [] });
    const label = this.labelOf(c);
    switch (g.how) {
      case "fixture":
        return this.fixture(c, g, label, false);
      case "cases":
        return this.suite(c, g, label);
      case "tests":
        return this.tests(c, g, label);
      case "proofs":
        return this.proofsCheck(c, g, label);
      case "selftest":
        return this.selftest(c, label);
      case "nvtest":
        return this.nvtest(c, g, label);
      case "heavy":
        return this.heavy(c, g, label);
      default:
        return this.command(c, g, label, false);
    }
  }

  private async fixture(c: Check, g: Grouped, label: string, heavy: boolean): Promise<Verdict> {
    // `needs = "af-unix"`: this platform's build has no Unix-domain transport, so the claim is not asked here.
    if (c.needs === "af-unix" && process.platform === "win32") {
      this.recordOwn(g, "green", new Map([[`file:${c.file}`, ""]]));
      return green;
    }
    const exe = await this.binary();
    if (typeof exe !== "string") return none(exe.fail);
    const args = [...(c.args ?? []), c.file ?? ""];
    const got = await this.recorded([exe, "run", ...args], ".", `nvs run ${args.join(" ")}`, { NOVIS_NO_FILE_CACHE: "1" });
    const { o } = got;
    const keys: Keyed = new Map(got.keys);
    const fail = judgeProgram(c, o, `native ${c.file} [${this.label(c.stage)}]`);
    if (heavy && this.graph) for (const [k, d] of heavyKeys(c, this.graph, await this.rustFiles())) keys.set(k, d);
    this.recordOwn(g, fail === "" ? "green" : "red", keys);
    return none(fail);
  }

  private async command(c: Check, g: Grouped, label: string, heavy: boolean): Promise<Verdict> {
    const argv = await this.argvOf(c);
    if (!Array.isArray(argv)) return none(argv.fail);
    if (measuresReleaseCli(c)) {
      const built = await this.releaseCli();
      if (built.code !== 0) return none(`the release build failed -- ${label}: ${firstErrLine(built)}`);
    }
    const cwd = c.cwd ?? ".";
    const { o, keys } = await this.recorded(argv, cwd, argv.join(" "), argv.includes(covwsNvs()) ? { NOVIS_NO_FILE_CACHE: "1" } : {});
    const own: Keyed = new Map(keys);
    // What records nothing of its reads: a command's `commandKeys`, and any `cargo` run everything.
    if (!heavy && c.kind === "command" && !(argv[0] === "bun" && argv[1] === "nv")) for (const [k, d] of commandKeys(c)) own.set(k, d);
    if (!heavy && c.kind === "cargo-named") own.set("*", "");
    if (heavy && this.graph) for (const [k, d] of heavyKeys(c, this.graph, await this.rustFiles())) own.set(k, d);
    else if (heavy) own.set("*", "");
    const fail = c.kind === "command" ? judgeCommand(c, o, label) : judgeTests(c, o, label, onDisk).fail;
    this.recordOwn(g, fail === "" ? "green" : "red", own);
    return none(fail);
  }

  private async heavy(c: Check, g: Grouped, label: string): Promise<Verdict> {
    if (PROGRAM_KINDS.has(c.kind)) return this.fixture(c, g, label, true);
    if (c.kind === "command") return this.command(c, g, label, true);
    // A release test or anything else cargo runs: cargo builds and runs it, and its footprint logs record.
    const args = c.kind === "nvs-suite" ? null : (c.args ?? []);
    if (args === null) {
      const exe = await this.binary();
      if (typeof exe !== "string") return none(exe.fail);
    }
    const argv = args === null ? [covwsNvs(), ...(c.args ?? [])] : ["cargo", ...args];
    const got = await this.recorded(argv, ".", argv.join(" "));
    const { o } = got;
    const keys: Keyed = new Map(got.keys);
    if (this.graph) for (const [k, d] of heavyKeys(c, this.graph, await this.rustFiles())) keys.set(k, d);
    else keys.set("*", "");
    const v = judgeTests(c, o, label, onDisk);
    this.recordOwn(g, v.fail === "" ? "green" : "red", keys);
    return v;
  }

  // ---- a suite of cases --------------------------------------------------------------------------

  private async suite(c: Check, g: Grouped, label: string): Promise<Verdict> {
    const want = new Set(this.lacking(c, g).map((a) => a.slice(5)));
    const todo = (g.cases ?? []).filter((p) => (this.picked(caseId(p)) || want.has(p)) && !this.casesDone.has(p));
    if (todo.length > 0) {
      const exe = await this.binary();
      if (typeof exe !== "string") return none(exe.fail);
      for (const p of todo) this.casesDone.add(p);
      this.say(`nvs test --cases: ${todo.length} case(s)`);
      const got = await recordCases(this.rec, exe, todo, { jobs: cpus().length || 4, batch: 128 });
      for (const p of todo) this.ran.add(caseId(p));
      this.caseOut += got.out;
    }
    return this.judgeSuite(c, g, label);
  }

  /** A suite's verdict from the store: every case of its trees green, each named case on disk and not
   * skipped, and `minPassing` cases passing, the last reported as `short`. */
  private judgeSuite(c: Check, g: Grouped, label = this.labelOf(c)): Verdict {
    const cases = g.cases ?? [];
    const red = cases.filter((p) => this.store.atom(caseId(p))?.verdict !== "green");
    if (red.length > 0) {
      const more = red.length > 3 ? `, +${red.length - 3} more` : "";
      const why = this.caseOut.split("\n").find((l) => l.startsWith(`FAIL ${red[0]}`));
      return none(`${label}: ${red.length} case(s) failed: ${red.slice(0, 3).join(", ")}${more}${why ? ` -- ${why.trim()}` : ""}`);
    }
    for (const kase of c.cases ?? []) {
      const want = kase.replaceAll("\\", "/");
      if (!onDisk(kase)) return none(`${label}: case ${want} is not written yet`);
      if (caseSkipped(this.store, want)) return none(`${label}: case ${want} was skipped, so nothing ran it`);
    }
    const passing = cases.filter((p) => caseSkipped(this.store, p) !== true).length;
    const floor = c.minPassing;
    return { fail: "", short: floor !== undefined && passing < floor ? `${label}: only ${passing} passing case(s), wanted at least ${floor}` : "" };
  }

  // ---- test binaries -----------------------------------------------------------------------------

  /** One whole run of the binary `<package> <kind> <target>`, once per sweep, recorded. */
  private runExe(name: string, exes: Map<string, TestExe[]>, threads: number): Promise<{ o: Outcome; verdict: AtomVerdict }> {
    let p = this.exeRuns.get(name);
    if (p === undefined) {
      p = (async () => {
        let found: { pkg: string; t: TestExe } | null = null;
        for (const [pkg, ts] of exes) for (const t of ts) if (`${pkg} ${t.kind} ${t.kind === "lib" ? t.target.replace(/-/g, "_") : t.target}` === name) found = { pkg, t };
        if (found === null) return { o: { code: -1, out: "", err: `no test executable in the workspace build is ${name}` }, verdict: "red" as AtomVerdict };
        const { pkg, t } = found;
        this.say(`${pkg}: ${t.target}`);
        const id = `test:${name}`;
        const rec = recordName(id);
        const env = { ...this.rec.env(rec), ...(await this.rec.cacheDir(rec)), CARGO_MANIFEST_DIR: t.dir, RUST_TEST_THREADS: String(threads), NO_COLOR: "1" };
        const o = await capture([t.exe], t.dir, env);
        const ext = await this.rec.keysOf(rec, [t.exe, covwsNvs()]);
        const verdict: AtomVerdict = o.code === 0 && ext ? "green" : "red";
        this.store.recordRun(id, { def: "", verdict, keys: testKeys(pkg, ext, name) });
        putTestGreen(this.store, name, verdict, o.out);
        if (verdict === "red") {
          const failed = o.out.split(/\r?\n/).filter((l) => l.startsWith("test ") && l.trimEnd().endsWith("FAILED")).map((l) => l.split(/\s+/)[1]);
          this.exeShown.set(name, failed.length > 0 ? `${failed.length} test(s) failed: ${failed.join(", ")}` : `exit ${o.code} -- ${firstErrLine(o)}`);
        }
        this.ran.add(id);
        return { o, verdict };
      })();
      this.exeRuns.set(name, p);
    }
    return p;
  }

  private async tests(c: Check, g: Grouped, label: string): Promise<Verdict> {
    const want = new Set(this.lacking(c, g).map((a) => a.slice(5)));
    const todo = (g.tests ?? []).filter((n) => this.picked(`test:${n}`) || want.has(n));
    if (todo.length > 0) {
      const exes = await this.testExes();
      if (!(exes instanceof Map)) return none(exes.fail);
      const width = Math.max(1, Math.min(todo.length, Math.floor((cpus().length || 4) / 2)));
      const threads = Math.max(1, Math.floor((cpus().length || 4) / width));
      await pool(todo, width, async (n) => {
        await this.runExe(n, exes, threads);
      });
    }
    return this.judgeTestCheck(c, g, label);
  }

  /** A test check's verdict: every binary it names green, and each test it names among the lines the
   * binaries printed, this sweep's run or the last green one the store keeps. */
  private judgeTestCheck(c: Check, g: Grouped, label = this.labelOf(c)): Verdict {
    const names = g.tests ?? [];
    for (const n of names) {
      if (this.store.atom(`test:${n}`)?.verdict === "green") continue;
      return none(`${label}: ${n} failed${this.failedTests(n)}`);
    }
    const tests = c.tests ?? [];
    if (tests.length === 0) return green;
    const text = names.map((n) => (testGreen(this.store, n)?.tests ?? []).join("\n")).join("\n");
    const missing = tests.find((t) => !text.includes(t));
    return none(missing === undefined ? "" : `${label}: test ${JSON.stringify(missing)} did not run`);
  }

  /** The failed tests this sweep's run of `name` named, as a suffix, or "". */
  private failedTests(name: string): string {
    const shown = this.exeShown.get(name);
    return shown ? ` -- ${shown}` : "";
  }

  // ---- the tools' own gate -----------------------------------------------------------------------

  private tool(atom: string, body: () => Promise<ToolRun>): Promise<ToolRun> {
    let p = this.toolRuns.get(atom);
    if (p === undefined) {
      p = body().then((r) => {
        this.ran.add(atom);
        return r;
      });
      this.toolRuns.set(atom, p);
    }
    return p;
  }

  private async selftest(c: Check, label: string): Promise<Verdict> {
    if (this.picked(NV_TSC)) {
      this.say("tsc --noEmit");
      const t = await this.tool(NV_TSC, () => runTsc(this.rec));
      if (t.verdict === "red") return none(judgeCommand(c, { code: 1, out: `${t.out}\nnv selftest: tsc failed with exit ${t.code}\n`, err: "" }, label) || `${label}: tsc failed`);
    }
    const files = nvTestFiles().filter((f) => this.picked(nvTestId(f)));
    const width = Math.max(1, Math.min(4, Math.floor((cpus().length || 4) / 4)));
    await pool(files, width, async (f) => {
      await this.tool(nvTestId(f), () => {
        this.say(`bun test ${f}`);
        return runNvTest(this.rec, f);
      });
    });
    const reds = nvTestFiles().filter((f) => this.store.atom(nvTestId(f))?.verdict !== "green");
    const out = reds.length === 0 ? "nv selftest: the types check\nnv selftest: every test passes\n 0 fail\n" : `nv selftest: the types check\nnv selftest: bun test failed in ${reds.join(", ")}\n`;
    return none(judgeCommand(c, { code: reds.length === 0 ? 0 : 1, out, err: "" }, label));
  }

  private async nvtest(c: Check, g: Grouped, label: string): Promise<Verdict> {
    const file = g.file!;
    if (!this.picked(nvTestId(file))) return green;
    const t = await this.tool(nvTestId(file), () => {
      this.say(`bun test ${file}`);
      return runNvTest(this.rec, file);
    });
    return none(judgeCommand(c, { code: t.code, out: t.out, err: "" }, label));
  }

  // ---- proofs groups -----------------------------------------------------------------------------

  private async proofsCheck(c: Check, g: Grouped, label: string): Promise<Verdict> {
    const groups = proofGroups(c);
    const batch = this.proofs;
    let o: Outcome;
    let keys: Keyed;
    if (groups !== null && batch !== undefined && groups.every((x) => batch.groups.includes(x))) {
      batch.run ??= this.recorded(["bun", "nv", "proofs", "--verify", ...batch.groups.flatMap((x) => ["--group", x])], ".", `bun nv proofs --verify over ${batch.groups.length} groups`);
      const got = await batch.run;
      o = proofOutcome(groups, got.o);
      keys = new Map(got.keys);
    } else {
      const got = await this.recorded(c.argv ?? [], c.cwd ?? ".", (c.argv ?? []).join(" "));
      o = got.o;
      keys = new Map(got.keys);
    }
    const fail = judgeCommand(c, o, label);
    this.recordOwn(g, fail === "" ? "green" : "red", keys);
    return none(fail);
  }

  // ---- the Linux legs ----------------------------------------------------------------------------

  /** Whether the change reaches a leg: its atom is new, red, owed, or holds a key the change moved. */
  legReached(leg: string): boolean {
    return this.picked(legId(leg));
  }

  /** Records a green leg, run with the floor gate open. */
  async legGreen(leg: string): Promise<void> {
    const keys: Keyed = this.graph ? heavyKeys(null, this.graph, await this.rustFiles()) : new Map([["*", ""]]);
    for (const t of LEG_TREES) keys.set(`tree:${t}`, "");
    this.store.recordRun(legId(leg), { def: LEG_DEF, verdict: "green", keys });
    this.ran.add(legId(leg));
  }

  /** Records a red leg: it stays selected until a run of it is green. */
  legRed(leg: string): void {
    this.store.setVerdict(legId(leg), "red");
    this.ran.add(legId(leg));
  }

  // ---- the end -----------------------------------------------------------------------------------

  /** Closes the store without moving its tree: for a caller that only asked what the change reaches. */
  discard(): void {
    this.rec.close();
    this.store.close();
  }

  /**
   * Moves the store's tree past the change, green or red, and closes the store. What this sweep ran, and
   * what a `bun nv` process it started recorded, counts as run; every other atom the change reached is
   * owed. The atoms of checks the plan no longer holds are forgotten. Returns how many atoms are owed.
   */
  close(): { owed: number } {
    try {
      const ids = new Set(this.plan.map((c) => c.id));
      for (const kind of ["check", "nv", "heavy"] as const) {
        for (const a of this.store.atoms(kind)) {
          const id = a.id.slice(kind.length + 1);
          if (kind === "heavy" && (LEGS as readonly string[]).includes(id)) continue;
          if (!ids.has(id)) this.store.removeAtom(a.id);
        }
      }
      const ran = new Set(this.ran);
      for (const a of this.store.atoms()) if (a.lastRun >= this.started && a.verdict !== "owed") ran.add(a.id);
      return advance(this.store, this.change, this.sel, ran, this.graph);
    } finally {
      this.rec.close();
      this.store.close();
    }
  }
}

/** What a leg's atom is defined by: nothing a plan writes. */
const LEG_DEF = "leg";

/**
 * Every check of `plan` whose atoms the store last recorded green, by id: what a status row and the goal
 * table start from before a sweep has judged anything. It reads no change, so it says what the last runs
 * found, not what the tree as it stands would.
 */
export async function lastGreen(plan: Check[]): Promise<Map<string, boolean>> {
  const store = new SelectStore();
  try {
    const verdicts = new Map(store.atoms().map((a) => [a.id, a.verdict]));
    const ctx: PlanContext = { graph: await metadata(), cases: caseFiles(), nvTests: nvTestFiles(), groupDirs: groupDirsOf(store), proofs: store.atoms("proof").map((a) => a.id.slice(6)) };
    const out = new Map<string, boolean>();
    for (const c of plan) {
      const g = grouped(c, ctx);
      if (!g.unknown && g.atoms.length > 0 && g.atoms.every((a) => verdicts.get(a) === "green")) out.set(c.id, true);
    }
    return out;
  } finally {
    store.close();
  }
}
