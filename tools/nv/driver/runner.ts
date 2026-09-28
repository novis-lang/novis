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
// - a fixture and a command find that same `nvs` in `NVS_BIN`, so a program or a script that starts
//   `nvs` itself runs this tree's build and never a `target/debug` one somebody built by hand;
// - a case runs through `nvs test --cases` (`recordCases`), shared with `nv verify`'s case trees;
// - a test binary runs whole in its package's directory, shared with `nv verify`'s `test` step, which
//   also keeps its last green lines for a check that names a test;
// - a `bun nv` command runs with `NV_READS_LOG`, so its reads and the modules it loaded are its footprint,
//   and with `NV_SELECT_NO_ADVANCE`, so a `bun nv proofs` it starts records its programs and leaves the
//   tree to the sweep;
// - the tools' `tsc` and each of their test files run as `select/nvtests.ts` runs them for `nv verify`;
// - a heavy check runs as it always has, with the footprint logs of what it starts, and then its twin
//   runs on the covws build and records which Rust code it used (`select/checks.ts` `heavyTwin`); fuzz
//   and TSan have none and hold `heavyKeys`;
// - a Linux leg is picked when a fixture or case it runs again is, and on the safety net's cadence every
//   heavy check and leg is picked (`OpenOptions.heavyAll`).
//
// A sweep shares every process between the checks that ask for the same one. Two checks naming one
// `argv` in one directory get one run, recorded under each check's atom, and each judges its own exit
// and `want` against it; the same holds for a case, a test binary, a tools test file and a heavy check's
// twin, which two heavy checks share when it runs the same work (`twinWork`). The
// `nv proofs --verify` checks of one tier share a run too, `--group` and `--only` forms alike: the first
// one reached starts one `nv proofs --verify` over every group the tier's reached checks name and every
// feature list they name (`--only-as`), under one check's time limit for each `PROOF_SCOPES_PER_LIMIT` of those scopes, and
// each check is judged on its own scopes' sections of what that run prints. A check the run printed no
// section for, because it died or was killed, runs again on its own (`accept.ts` `proofResult`).
//
// One covws build serves every fixture, suite and `{nvs}` command, and one `cargo test --no-run` every
// test binary the sweep runs. That build names its binaries with target filters (`sweepTestFilters`),
// never a `-p`, so a change to a library every binary links relinks only the binaries the sweep runs; a
// sweep over every atom, or one with no workspace graph, builds every binary. A check that measures the
// release CLI gets `cargo build --release -p nvs-cli` first, since nothing else builds it.

import { mkdtempSync, rmSync } from "node:fs";
import { cpus } from "node:os";
import { join } from "node:path";
import { allTestBinaries, metadata, type Graph, targetFilters } from "../keys/graph.ts";
import { covwsCargo, covwsNvs } from "../lib/covws.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { linked, releaseCli } from "../lib/relink.ts";
import { recordName } from "../proofs/run.ts";
import { caseId, nvTestFiles, nvTestId } from "../select/atoms.ts";
import { commandKeys, crateKeys, grouped, type Grouped, heavyKeys, heavyTwin, LEG_KEYS, legAtoms, LEGS, legId, onDisk, planContext, predicted, type Twin, twinBinaries, twinWork } from "../select/checks.ts";
import { PLATFORM_ONLY, WILD } from "../select/keys.ts";
import { NO_ADVANCE_ENV, advance, caseSkipped, fullChange, pool, putTestGreen, Recorder, recordCases, testGreen } from "../select/record.ts";
import { NV_TSC, runNvTest, runTsc, type ToolRun } from "../select/nvtests.ts";
import { type ChangeSet, computeChange, discover, query, rustFiles, type Selection } from "../select/select.ts";
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
  proofBatch,
  proofBatchArgv,
  type ProofBatch,
  proofLimitMs,
  proofResult,
  releaseBuildArgv,
  type TestExe,
  testExecutables,
  type Verdict,
} from "./accept.ts";

const TIMEOUT_MS = 1800 * 1000;

/** Runs `argv` under `timeoutMs`, and turns a program that cannot start into exit -1 with the reason on
 * stderr. Whatever it started and left behind is killed with it. Aborting `stop` kills it too, and
 * the outcome is exit -1. */
async function capture(argv: string[], cwd: string, env?: Record<string, string>, timeoutMs: number = TIMEOUT_MS, stop?: AbortSignal): Promise<Outcome> {
  try {
    const r = await run(argv, { cwd, timeoutMs, reap: true, ...(env ? { env } : {}), ...(stop ? { signal: stop } : {}) });
    if (r.aborted) return { code: -1, out: r.stdout, err: "stopped: the sweep ended before it" };
    return { code: r.timedOut ? -1 : r.code, out: r.stdout, err: r.timedOut ? `timed out after ${timeoutMs / 1000}s` : r.stderr };
  } catch (e) {
    return { code: -1, out: "", err: String((e as Error).message ?? e) };
  }
}

const none = (fail: string): Verdict => ({ fail, short: "" });
const green: Verdict = { fail: "", short: "" };

/**
 * The variables a check's command runs with beyond the recorder's. A command that runs the pipeline's
 * `nvs` itself has the compile cache off. A `command` check that does not measure the release CLI has
 * that `nvs` in `NVS_BIN`, so a script that starts `nvs` itself, as the editor's headless suites do,
 * runs this tree's build and records its footprint, and never falls back to a `target/debug` build that
 * may be missing or old. `exe` is the pipeline's `nvs`, or null when it did not build.
 */
export function commandEnv(c: Check, argv: string[], exe: string | null): Record<string, string> {
  const env: Record<string, string> = argv.includes(covwsNvs()) ? { NOVIS_NO_FILE_CACHE: "1" } : {};
  if (exe !== null && c.kind === "command" && !measuresReleaseCli(c)) env.NVS_BIN = exe;
  return env;
}

/**
 * The target filters the sweep's `cargo test --no-run` takes to build the test binaries `names`, or
 * `null` for a build of every workspace test binary: a sweep over every atom, one with no workspace
 * graph to name the binaries, and one whose `names` are every binary the graph names. A name the graph
 * does not have is left out of the filters, and its run is red as it is off a build of every binary.
 */
export function sweepTestFilters(names: Iterable<string>, graph: Graph | null, full: boolean): string[] | null {
  if (full || graph === null) return null;
  const want = new Set(names);
  const every = allTestBinaries(graph);
  if (every.every((n) => want.has(n))) return null;
  return targetFilters(every.filter((n) => want.has(n)));
}

export interface OpenOptions {
  /** Pick every atom of every check, whatever changed. */
  full: boolean;
  /** Called with each process before it starts. */
  onRun?: (what: string) => void;
  /** A line about what the sweep could not read, which never stops it. */
  say?: (line: string) => void;
  /** The store to read and record in, which the caller closes; the default store, closed by `close`,
   * when absent. */
  store?: SelectStore;
  /** Pick every heavy check and both Linux legs, whatever changed: the loop's safety net for their keys,
   * on its cadence of goal ends. */
  heavyAll?: boolean;
}

/** One sweep over a plan: the change it reads, what that reaches, and the runs it makes. */
export class PlanSweep {
  /** Every atom this sweep ran and recorded. */
  readonly ran = new Set<string>();
  private readonly started = Date.now();
  private cli: Promise<string | { fail: string }> | undefined;
  private exes: Promise<Map<string, TestExe[]> | { fail: string }> | undefined;
  /** The test binaries `exes` builds, or `null` when it builds every one. */
  private exesFor: Set<string> | null = null;
  private release: Promise<Outcome> | undefined;
  private files: Promise<string[]> | undefined;
  private readonly shared = new Map<string, Promise<{ o: Outcome; keys: Keyed; parts: Map<string, Keyed> }>>();
  private readonly exeRuns = new Map<string, Promise<{ o: Outcome; verdict: AtomVerdict }>>();
  private readonly twinRuns = new Map<string, Promise<Keyed | null>>();
  private readonly toolRuns = new Map<string, Promise<ToolRun>>();
  private readonly casesDone = new Set<string>();
  private readonly exeShown = new Map<string, string>();
  private caseOut = "";
  private proofs: { batch: ProofBatch; run?: Promise<{ o: Outcome; keys: Keyed; parts: Map<string, Keyed> }> } | null = null;
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
    const store = o.store ?? new SelectStore();
    const graph = await metadata();
    let change: ChangeSet;
    try {
      change = store.base() === null ? await fullChange() : await computeChange(store, { graph });
    } catch (e) {
      change = await fullChange(ROOT, `the recorded tree could not be read: ${(e as Error).message.split("\n")[0]}`);
    }
    const ctx = planContext(store, graph);
    const groups = new Map(plan.map((c) => [c.id, grouped(c, ctx)]));
    // Every atom the tree names is discovered, whether or not a check of this plan runs it, so one the
    // store has never recorded is selected as new.
    const found = discover(graph);
    const discovered = new Set(found.atoms);
    const defs = new Map<string, string>();
    for (const g of groups.values()) {
      for (const a of g.atoms) discovered.add(a);
      if (g.own) {
        discovered.add(g.own.id);
        defs.set(g.own.id, g.own.def);
      }
    }
    for (const leg of LEGS) {
      discovered.add(legId(leg));
      defs.set(legId(leg), LEG_DEF);
    }
    const sel = query(store, change, { discovered: [...discovered], complete: found.complete, defs, ran: twinBinaries(plan, graph) });
    // A leg runs every fixture and suite of the plan again on Linux, so a change that reaches one of them
    // reaches both legs, for the same reason.
    const by = legAtoms(plan, groups)
      .map((a) => sel.selected.get(a))
      .find((s) => s !== undefined);
    const heavy = [...groups.values()].flatMap((g) => (g.how === "heavy" && g.own ? [g.own.id] : []));
    for (const id of LEGS.map(legId)) if (by && !sel.selected.has(id)) sel.selected.set(id, { id, kind: "heavy", why: by.why, keys: by.keys });
    if (o.heavyAll) {
      for (const id of [...heavy, ...LEGS.map(legId)]) {
        if (!sel.selected.has(id)) sel.selected.set(id, { id, kind: "heavy", why: "key", keys: [{ key: HEAVY_ALL, origin: { path: "", how: "cadence" } }] });
      }
    }
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

  /** Every test binary a `tests` check of the plan runs this sweep: one whose atom is picked, and one a
   * check names a test of with no green run kept. */
  private testsWanted(): string[] {
    const out = new Set<string>();
    for (const c of this.plan) {
      const g = this.groups.get(c.id);
      if (g?.how !== "tests") continue;
      const want = new Set(this.lacking(c, g).map((a) => a.slice(5)));
      for (const n of g.tests ?? []) if (this.picked(`test:${n}`) || want.has(n)) out.add(n);
    }
    return [...out];
  }

  /** The test executables `todo` needs, off one `cargo test --no-run` over every binary the sweep runs.
   * A binary that build left out is built by a second one over both. */
  private testExes(todo: string[]): Promise<Map<string, TestExe[]> | { fail: string }> {
    if (this.exes !== undefined && (this.exesFor === null || todo.every((n) => this.exesFor!.has(n)))) return this.exes;
    const names = new Set([...(this.exesFor ?? []), ...this.testsWanted(), ...todo]);
    const filters = sweepTestFilters(names, this.graph, this.o.full);
    this.exesFor = filters === null ? null : names;
    this.exes = (async () => {
      if (filters !== null && filters.length === 0) return new Map<string, TestExe[]>();
      this.say(`cargo test --no-run${filters === null ? "" : ` ${filters.join(" ")}`} (covws)`);
      const { env, args } = covwsCargo();
      const r = await capture(["cargo", "test", "--no-run", ...args, ...(filters ?? []), "--message-format=json"], ROOT, env);
      if (r.code !== 0) return { fail: `the workspace test build failed -- ${firstErrLine(r)}` };
      return testExecutables(r.out);
    })();
    return this.exes;
  }

  /** Builds the release `nvs` once per sweep. A build that `stop` killed is forgotten, so a later
   * caller builds it again. */
  private releaseCli(stop?: AbortSignal): Promise<Outcome> {
    this.release ??= (async () => {
      this.say("cargo build --release -p nvs-cli");
      // An editor running `nvs lsp` out of this tree holds the binary the link replaces;
      // `tools/nv/lib/relink.ts` moves it aside so the retry lands.
      const r = await linked(releaseCli(), () => capture(["cargo", "build", "--release", "-p", "nvs-cli"], ROOT, undefined, TIMEOUT_MS, stop), (r) => r.err);
      if (stop?.aborted) this.release = undefined;
      return r;
    })();
    return this.release;
  }

  private rustFiles(): Promise<string[]> {
    this.files ??= rustFiles();
    return this.files;
  }

  // ---- recorded runs -----------------------------------------------------------------------------

  /** One recorded run of `argv` in `cwd`, shared by every check that asks for the same one: what it
   * printed, and what it was seen to use. A `bun nv` process's reads are recorded besides, and for each of
   * `parts` also as that part of the process read it (`readLog`), under `parts`. It runs under one
   * check's time limit unless `timeoutMs` names another. */
  private recorded(argv: string[], cwd: string, what: string, extra: Record<string, string> = {}, parts: string[] = [], timeoutMs: number = TIMEOUT_MS): Promise<{ o: Outcome; keys: Keyed; parts: Map<string, Keyed> }> {
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
        const o = await capture(argv, join(ROOT, cwd), env, timeoutMs);
        const programs = (await this.rec.keysOf(name, [covwsNvs()]))?.keys ?? new Map<string, string>();
        const keyed = (part?: string): Keyed => {
          const keys: Keyed = nv ? nvKeys(log, part) : new Map();
          for (const [k, d] of programs) keys.set(k, d);
          return keys;
        };
        return { o, keys: keyed(), parts: new Map(parts.map((x) => [x, keyed(x)])) };
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

  /** Names the checks the sweep is about to reach, so the first `nv proofs --verify` check among them
   * runs every scope of the batch `proofBatch` forms in one process. */
  batch(checks: Check[]): void {
    const b = proofBatch(checks);
    this.proofs = b === null ? null : { batch: b };
  }

  /**
   * Builds what the release checks `checks` will build, one cargo run after another: the release `nvs`
   * for a check that measures it, and each release test with `--no-run`. The sweep calls it when its
   * goal fixtures tier starts, so the builds run beside that tier and the overlap commands, and the
   * checks later find everything built. Aborting `stop` kills the build in progress, and no further
   * build starts. A build that fails is left for its check to build again and report, except the
   * release `nvs`, whose one outcome every check that measures it shares.
   */
  prebuild(checks: Check[], stop: AbortSignal): Promise<void> {
    return (async () => {
      if (checks.some(measuresReleaseCli) && !stop.aborted) await this.releaseCli(stop);
      const seen = new Set<string>();
      for (const c of checks) {
        const argv = releaseBuildArgv(c);
        if (stop.aborted) return;
        if (argv === null || seen.has(argv.join("\0"))) continue;
        seen.add(argv.join("\0"));
        this.say(`${argv.join(" ")} (built beside the goal fixtures and the overlap commands)`);
        await capture(argv, ROOT, undefined, TIMEOUT_MS, stop);
      }
    })().catch(() => undefined);
  }

  /** Runs the picked atoms of `c`, records each, and judges it. */
  async check(c: Check): Promise<Verdict> {
    const g = this.groups.get(c.id) ?? grouped(c, { graph: this.graph, cases: [], nvTests: [], groupDirs: new Map(), proofs: [] });
    const label = this.labelOf(c);
    switch (g.how) {
      case "fixture":
        return this.fixture(c, g, label);
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

  private async fixture(c: Check, g: Grouped, label: string): Promise<Verdict> {
    // `needs = "af-unix"`: this platform's build has no Unix-domain transport, so the claim is not asked here.
    if (c.needs === "af-unix" && process.platform === "win32") {
      this.recordOwn(g, "green", new Map([[`file:${c.file}`, ""]]));
      return green;
    }
    const exe = await this.binary();
    if (typeof exe !== "string") return none(exe.fail);
    const args = [...(c.args ?? []), c.file ?? ""];
    const got = await this.recorded([exe, "run", ...args], ".", `nvs run ${args.join(" ")}`, { NOVIS_NO_FILE_CACHE: "1", NVS_BIN: exe });
    const { o } = got;
    const keys: Keyed = new Map(got.keys);
    const fail = judgeProgram(c, o, `native ${c.file} [${this.label(c.stage)}]`);
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
    const exe = c.kind === "command" && !measuresReleaseCli(c) ? await this.binary() : null;
    const extra = commandEnv(c, argv, typeof exe === "string" ? exe : null);
    const { o, keys } = await this.recorded(argv, cwd, argv.join(" "), extra);
    const own: Keyed = new Map(keys);
    // What records nothing of its reads: a command's `commandKeys`, and any `cargo` run everything.
    if (!heavy && c.kind === "command" && !(argv[0] === "bun" && argv[1] === "nv")) for (const [k, d] of commandKeys(c)) own.set(k, d);
    if (!heavy && c.kind === "cargo-named") own.set("*", "");
    if (heavy) for (const [k, d] of await this.heavyKeysOf(c)) own.set(k, d);
    const fail = c.kind === "command" ? judgeCommand(c, o, label) : judgeTests(c, o, label, onDisk).fail;
    this.recordOwn(g, fail === "" ? "green" : "red", own);
    return none(fail);
  }

  private async heavy(c: Check, g: Grouped, label: string): Promise<Verdict> {
    // A heavy fixture is one never memoized, and it runs on the covws build like any other.
    if (PROGRAM_KINDS.has(c.kind)) return this.fixture(c, g, label);
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
    for (const [k, d] of await this.heavyKeysOf(c)) keys.set(k, d);
    const v = judgeTests(c, o, label, onDisk);
    this.recordOwn(g, v.fail === "" ? "green" : "red", keys);
    return v;
  }

  /**
   * What heavy check `c` holds besides what its own run recorded: its twin's record and the keys for what
   * the twin cannot reach, the crates fuzz and TSan are predicted to build, or nothing for a check that
   * ran on the covws build itself. A twin that cannot run or be read gives `*`, which the store drops
   * again at the next run whose twin was read.
   */
  private async heavyKeysOf(c: Check): Promise<Keyed> {
    if (this.graph === null) return new Map([[WILD, ""]]);
    if (predicted(c)) return new Map([...heavyKeys(c, this.graph, await this.rustFiles()), [PLATFORM_ONLY, ""]]);
    const twin = heavyTwin(c, this.graph);
    if (twin === null) return new Map();
    const seen = await this.twin(c, twin);
    if (seen === null) {
      this.say(`the twin of ${this.labelOf(c)} could not be read: every change reaches it until a twin is`);
      return new Map([[WILD, ""]]);
    }
    for (const k of twin.held) seen.set(k, "");
    if (twin.crates.length > 0) for (const [k, d] of crateKeys(twin.crates, [], this.graph, await this.rustFiles())) seen.set(k, d);
    return seen;
  }

  /**
   * One run of `twin` on the covws build, recorded: each of its test binaries with its harness arguments,
   * then its command with `{nvs}` as the covws `nvs`. Its verdict counts for nothing, since a release
   * figure is not a debug one, but a command that fails may have stopped short, so it gives null, as does
   * a binary the build did not make or a record that cannot be read. Checks whose twins run the same work
   * (`twinWork`) share one run, and each gets its own copy of the keys it recorded.
   */
  private async twin(c: Check, twin: Twin): Promise<Keyed | null> {
    const key = twinWork(twin);
    let p = this.twinRuns.get(key);
    if (p === undefined) {
      p = this.runTwin(c, twin);
      this.twinRuns.set(key, p);
    }
    const keys = await p;
    return keys === null ? null : new Map(keys);
  }

  /** Runs the twin's work once, for `twin` to share; its log lines name `c`, the first check that asked. */
  private async runTwin(c: Check, twin: Twin): Promise<Keyed | null> {
    const keys: Keyed = new Map();
    const scratch = mkdtempSync(join(this.rec.dir, "twin-"));
    const env = Object.fromEntries(Object.entries(twin.env).map(([k, v]) => [k, v.replaceAll("{scratch}", scratch)]));
    try {
      if (twin.tests.length > 0) {
        const exes = await this.testExes(twin.tests.map((t) => t.name));
        if (!(exes instanceof Map)) return null;
        for (const t of twin.tests) {
          const found = exeOf(t.name, exes);
          if (found === null) return null;
          this.say(`twin of ${c.id}: ${t.name}${t.args.length ? ` ${t.args.join(" ")}` : ""} (covws)`);
          // Short, as `recorded` names its runs: the name is in the profile's path twice.
          const rec = `tw${++this.runs}`;
          await capture([found.t.exe, ...t.args], found.t.dir, { ...this.rec.env(rec), ...(await this.rec.cacheDir(rec)), CARGO_MANIFEST_DIR: found.t.dir, NO_COLOR: "1", ...env });
          const ext = await this.rec.keysOf(rec, [found.t.exe, covwsNvs()]);
          if (!ext) return null;
          for (const [k, d] of testKeys(ext, t.name)) keys.set(k, d);
        }
      }
      if (twin.argv.length > 0) {
        const exe = await this.binary();
        if (typeof exe !== "string") return null;
        const argv = twin.argv.map((a) => (a === "{nvs}" ? exe : a));
        const got = await this.recorded(argv, ".", `twin of ${c.id}: ${argv.join(" ")}`, { NVS_BIN: exe, ...env });
        if (got.o.code !== 0) return null;
        for (const [k, d] of got.keys) keys.set(k, d);
      }
    } finally {
      rmSync(scratch, { recursive: true, force: true });
    }
    return keys;
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
        const found = exeOf(name, exes);
        if (found === null) return { o: { code: -1, out: "", err: `no test executable in the workspace build is ${name}` }, verdict: "red" as AtomVerdict };
        const { pkg, t } = found;
        this.say(`${pkg}: ${t.target}`);
        const id = `test:${name}`;
        const rec = recordName(id);
        const env = { ...this.rec.env(rec), ...(await this.rec.cacheDir(rec)), CARGO_MANIFEST_DIR: t.dir, RUST_TEST_THREADS: String(threads), NO_COLOR: "1" };
        const o = await capture([t.exe], t.dir, env);
        const ext = await this.rec.keysOf(rec, [t.exe, covwsNvs()]);
        const verdict: AtomVerdict = o.code === 0 && ext ? "green" : "red";
        this.store.recordRun(id, { def: "", verdict, keys: testKeys(ext, name) });
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
      const exes = await this.testExes(todo);
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

  /** A proofs check: judged on the batch's one run while it is in the batch, and on a run of its own
   * otherwise, or when the batch printed no verdict for it (`proofResult`). */
  private async proofsCheck(c: Check, g: Grouped, label: string): Promise<Verdict> {
    const p = this.proofs;
    const parts = p?.batch.parts.get(c.id);
    const shared =
      p === null || parts === undefined
        ? null
        : () => {
            const { batch } = p;
            const lists = batch.named.length === 0 ? "" : ` and ${batch.named.length} feature list(s)`;
            const scopes = [...batch.groups, ...batch.named.map((n) => n.label)];
            p.run ??= this.recorded(proofBatchArgv(batch), ".", `bun nv proofs --verify over ${batch.groups.length} group(s)${lists} in one process`, {}, scopes, proofLimitMs(scopes.length, TIMEOUT_MS));
            return p.run;
          };
    const { o, keys } = await proofResult(parts ?? [], shared, (why) => this.recorded(c.argv ?? [], c.cwd ?? ".", `${(c.argv ?? []).join(" ")}${why}`));
    const fail = judgeCommand(c, o, label);
    this.recordOwn(g, fail === "" ? "green" : "red", keys);
    return none(fail);
  }

  // ---- the Linux legs ----------------------------------------------------------------------------

  /** Whether the change reaches a leg: its atom is new, red, owed, or holds a key the change moved. */
  legReached(leg: string): boolean {
    return this.picked(legId(leg));
  }

  /** Records a green leg, run with the floor gate open. What it ran is the plan's fixtures and suites,
   * whose own atoms select it (`open`), so its own atom holds only `LEG_KEYS`. */
  async legGreen(leg: string): Promise<void> {
    const keys: Keyed = new Map(LEG_KEYS.map((k) => [k, ""]));
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
    if (this.o.store === undefined) this.store.close();
  }

  /**
   * Moves the store's tree past the change, green or red, and closes the store. What this sweep ran, and
   * what a `bun nv` process it started recorded, counts as run; every other atom the change reached is
   * owed. The atoms of checks the plan no longer holds are forgotten. Returns how many atoms are owed,
   * how many of each kind ran, and every atom marked diverged, with why.
   */
  close(): { owed: number; ran: Record<string, number>; diverged: Map<string, string> } {
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
      const kinds: Record<string, number> = {};
      for (const id of ran) kinds[id.slice(0, id.indexOf(":"))] = (kinds[id.slice(0, id.indexOf(":"))] ?? 0) + 1;
      const owed = advance(this.store, this.change, this.sel, ran, this.graph).owed;
      return { owed, ran: kinds, diverged: this.store.divergences() };
    } finally {
      this.rec.close();
      if (this.o.store === undefined) this.store.close();
    }
  }
}

/** What a leg's atom is defined by: nothing a plan writes, only what its keys are read from. */
const LEG_DEF = "leg: selected by the fixtures and suites it runs";

/** The key a heavy atom is picked under on the safety net's cadence of goal ends. */
const HEAVY_ALL = "cadence:every heavy check";

/** The test executable `<package> <kind> <target>` names in `exes`, with its package, or null. */
function exeOf(name: string, exes: Map<string, TestExe[]>): { pkg: string; t: TestExe } | null {
  for (const [pkg, ts] of exes) for (const t of ts) if (`${pkg} ${t.kind} ${t.kind === "lib" ? t.target.replace(/-/g, "_") : t.target}` === name) return { pkg, t };
  return null;
}

/**
 * Every check of `plan` whose atoms the store last recorded green, by id: what a status row and the goal
 * table start from before a sweep has judged anything. It reads no change, so it says what the last runs
 * found, not what the tree as it stands would.
 */
export async function lastGreen(plan: Check[]): Promise<Map<string, boolean>> {
  const store = new SelectStore();
  try {
    const verdicts = new Map(store.atoms().map((a) => [a.id, a.verdict]));
    const ctx = planContext(store, await metadata());
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
