// Acceptance: runs a goal's checks and the floor's, and judges each by exit code and output alone. It
// ports `crates/looper-engine/src/accept.rs`. The machine outranks the claim, so nothing here reads
// what a session said.
//
// - `acceptance` runs the floor first, then the goal's checks by stage, then the reach checks. While a
//   stage is open, only the checks of stages up to it run, because a later stage's checks are red by
//   design. Reach checks, and checks of later stages, run only once every stage is done and every other
//   check is green. Until then they are reported as deferred.
// - A check with `run` is a command, run as a `check` job of the project's job store, so a timeout
//   kills everything it started (`../jobs/spawn.ts`). Its process gets the kit's environment with
//   `[checks] env` laid over it, then the loop's variables, then the check's own `env`. A check that
//   names no `timeout` takes `[checks] timeout`.
// - Checks that share a command share one run (decision 0025). `runReported` groups the checks it is
//   handed by argv, resolved `cwd` and own `env`, and a check with no `env` shares with one whose `env`
//   is empty. A group's command runs once, with the longest timeout of the checks it judges, and each
//   check is judged apart on that one output. Groups run in the order of their first check. A check
//   with only `file` shares nothing and is judged alone.
// - Each check a command judged gets that run's whole output in its own `<log dir>/<id>.log`. The log
//   of a shared run names the other checks it judged.
// - A command's `output_*` matchers read its stdout followed by its stderr as one text. A check with
//   only `file` judges the file: it must exist and hold `min_bytes`, and its text stands for the output.
// - A command is judged once it exits, not once its output closes. The output gets `LINGER_S` to close
//   after the command exits. Then the rest of the tree is killed, and the log says so. A command that
//   times out keeps the output it wrote before it was killed.
// - A group's command holds one heavy-job slot for its run only (`../jobs/slots.ts`), and a wait for
//   one is said on stderr.
//
// A command check can be the unit `check:<id>` in the project's unit store, the one verify keeps. What
// its last green rests on decides whether it is one (`Rests`):
//
// - **cargo**: a `cargo test` with no `cwd` that names a target selector, such as `--test`, and no
//   `--doc`, `--manifest-path` or `-Z` flag. It rests on what its test binaries read, as the cargo
//   observer finds it (`cargo.ts`), and on its `inputs` too when it has them.
// - **declared**: any other command check with `inputs`. It rests on its `inputs`.
// - **whole**: every other check. It is no unit and always runs.
//
// A unit's definition is the whole check, the project's root, and `[checks] timeout` and `env`. A cargo
// unit's adds the observer's version. The loop's variables stay out, because they change every
// session. A unit runs only when a change reaches it, and otherwise is green as kept, with no log
// written. A kept unit takes no part in its group's run, and a group left with no check to judge does
// not run. `acceptance` drops the units of checks that are no longer units.
//
// A group with a cargo unit to run first lists the tree and digests all of it. Then it builds the
// check's binaries without running them, as verify builds a cargo step, with the group's timeout and a
// heavy-job slot. Each due cargo unit is recorded on the union of the binaries' footprints. So a file
// edited during the build or the run voids the result. A build that fails or lists no binary records
// the whole tree, and the check's own run shows cargo's errors in its log.
//
// `Units.all` runs every unit, as `check --all` does. A unit that is red although no change reached it,
// and whose result is not void, is added to the store's selection-miss list. `acceptance` takes the
// results of an every-session sweep's checks that are not due in `counted`, and does not run those.
//
// A run reports itself to a `Progress` sink as it goes: the whole run so far, one `Report` per check in
// the order the checks run, first with none started, then again as each group starts, as its command's
// job starts, and as it ends. A report names that job, so a reader can follow the check's progress.
//
// # The bounds it keeps
//
// - A command's stdout and stderr, and a file check's text, each keep their first and their last
//   `OUTPUT_HALF` bytes (`output.ts`).
// - One run's output is alive at a time. A group's checks are judged and logged as soon as it ends,
//   and its output is dropped before the next group runs.
// - A run's reports hold one entry per check it runs.

import { closeSync, existsSync, mkdirSync, openSync, readSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { type Checks, checkEnv } from "../goal/config.ts";
import type { Check, Goal } from "../goal/model.ts";
import { type Lease, looperHome, SLOT_VAR, takeSlots } from "../jobs/slots.ts";
import { startJob } from "../jobs/spawn.ts";
import { KILLED_CODE, type Kind, storeOf } from "../jobs/store.ts";
import { Bounded } from "./output.ts";
import { compare, declared, type Footprint, footprintOf, isEmpty, Store, Tree, type UnitDef, union, unitDef, wholeTree } from "./select.ts";
import { build, isCargoTest, splitCargo } from "./verify.ts";

/** How long a command's output may stay open after the command has exited, in seconds. */
export const LINGER_S = 10;

/** The bytes a file check's text is read in. */
const READ_CHUNK = 64 * 1024;

/** One check's result. */
export interface Verdict {
  id: string;
  green: boolean;
  /** Why it is red, in one line; empty when green. */
  why: string;
  /** The log of its last run, when it ran a command. */
  log: string | null;
  /** Whether it did not run because its last green still holds. */
  kept: boolean;
  /** Whether it is red although no change reached it since its last green, which only `Units.all` finds. */
  missed: boolean;
}

/** The project's unit store, which keeps the units of command checks, and whether every check runs. */
export interface Units {
  /** The store's folder. */
  store: string;
  /** Whether a unit whose last green still holds runs anyway, as `check --all` runs it. */
  all: boolean;
}

/** What every check of a run gets: `[checks]`, and the loop's variables. */
export interface Project {
  checks: Checks;
  vars: Record<string, string | undefined>;
}

/** The result of one acceptance run. */
export interface Acceptance {
  /** In the order the checks ran. */
  verdicts: Verdict[];
  /** The ids of the checks not run because a stage is open or another check is red. */
  deferred: string[];
  green: boolean;
}

/** One check of a run, as the `Progress` sink sees it. */
export interface Report {
  id: string;
  /** The check's name, or empty when it has none. */
  name: string;
  /** When it started, in ms since the Unix epoch, once it has. */
  started: number | null;
  /** Whether it passed, once it has ended. */
  green: boolean | null;
  /** How long it ran, once it has ended. */
  duration_ms: number | null;
  /** The id of the job its command runs as, once that job has started. Null for a check with no command. */
  job: string | null;
}

/** Where a run reports itself: the whole run so far, each time it changes. */
export type Progress = (run: Report[]) => void;

/** A sink that reports nowhere. */
export function unreported(_: Report[]): void {}

/**
 * Runs the floor and the goal's checks in their order, from the project root `root`, keeping the units
 * of command checks as `units` says, and reporting the run to `progress`. A check named in `counted`
 * does not run: its verdict is the result it maps to, with no log.
 */
export async function acceptance(
  root: string,
  goal: Goal,
  floor: Check[],
  logDir: string,
  units: Units,
  project: Project,
  progress: Progress,
  counted: ReadonlyMap<string, boolean> = new Map(),
): Promise<Acceptance> {
  const ids = [...floor, ...goal.front.checks].filter((c) => rests(c) !== "whole").map((c) => unitId(c.id));
  if (ids.length > 0 || existsSync(units.store)) Store.open(units.store).retain("check:", ids);
  const open = goal.front.stages.find((s) => s.status !== "done")?.id;
  const due = (c: Check) => open === undefined || c.stage === undefined || c.stage <= open;
  const reach = (c: Check) => c.reach === true;
  const staged = goal.front.checks.filter((c) => !reach(c) && due(c)).sort((a, b) => (a.stage ?? 0) - (b.stage ?? 0));
  const ordinary = [...floor.filter((c) => !reach(c)), ...staged];
  const last = [...floor.filter(reach), ...goal.front.checks.filter((c) => reach(c) || !due(c))];
  const run: Report[] = [];
  const [toRun, verdicts] = count(ordinary, counted);
  verdicts.push(...(await runReported(root, toRun, logDir, units, project, progress, run)));
  let deferred: string[] = [];
  if (open === undefined && verdicts.every((v) => v.green)) {
    const [lastToRun, kept] = count(last, counted);
    verdicts.push(...kept);
    verdicts.push(...(await runReported(root, lastToRun, logDir, units, project, progress, run)));
  } else {
    deferred = last.map((c) => c.id);
  }
  return { verdicts, deferred, green: deferred.length === 0 && verdicts.every((v) => v.green) };
}

/** Splits `checks` into the ones to run and the verdicts of the ones `counted` names. */
function count(checks: Check[], counted: ReadonlyMap<string, boolean>): [Check[], Verdict[]] {
  const verdicts: Verdict[] = [];
  const toRun: Check[] = [];
  for (const c of checks) {
    const green = counted.get(c.id);
    if (green === undefined) toRun.push(c);
    else verdicts.push({ ...verdict(c, green ? "" : "red at its last run, and not due this sweep", null), green });
  }
  return [toRun, verdicts];
}

/** The floor's and the goal's checks whose id, or whose name ignoring case, contains `text`. */
export function named(floor: Check[], goal: Goal, text: string): Check[] {
  const lower = text.toLowerCase();
  return [...floor, ...goal.front.checks].filter((c) => c.id.includes(text) || (c.name?.toLowerCase().includes(lower) ?? false));
}

/**
 * Runs `checks` and judges each, a unit only when a change reaches it in the store `units` names, or
 * always with `Units.all`. Checks that share a command share one run, and run in the order of `groups`,
 * which is the order of the verdicts. Adds them to the reports in `run`, and reports the whole run to
 * `progress` before the first group starts and as each starts and ends.
 */
export async function runReported(
  root: string,
  checks: Check[],
  logDir: string,
  units: Units,
  project: Project,
  progress: Progress,
  run: Report[],
): Promise<Verdict[]> {
  const opened: { store?: Store } = {};
  const order = groups(root, checks);
  let at = run.length;
  for (const i of order.flat()) {
    const c = checks[i] as Check;
    run.push({ id: c.id, name: c.name ?? "", started: null, green: null, duration_ms: null, job: null });
  }
  progress(run);
  const verdicts: Verdict[] = [];
  for (const group of order) {
    const members = group.map((i) => checks[i] as Check);
    const started = Date.now();
    const reports = run.slice(at, at + members.length);
    for (const r of reports) r.started = started;
    progress(run);
    const onJob = (job: string) => {
      for (const r of reports) r.job = job;
      progress(run);
    };
    const judged = await runGroup(root, members, logDir, units, project, opened, onJob);
    const ms = Date.now() - started;
    reports.forEach((r, k) => {
      r.green = judged[k]?.green ?? null;
      r.duration_ms = ms;
    });
    progress(run);
    at += members.length;
    verdicts.push(...judged);
  }
  return verdicts;
}

/**
 * `checks` as groups of indexes, each group the checks that share one run: the same argv, the same
 * directory, and the same own `env`, where none is the same as an empty one. Groups are in the order
 * of their first check, and each keeps its checks in their order. A check with no `run` is a group
 * alone.
 */
export function groups(root: string, checks: Check[]): number[][] {
  const order: number[][] = [];
  const seen = new Map<string, number>();
  checks.forEach((c, i) => {
    if (c.run.length === 0) {
      order.push([i]);
      return;
    }
    const env = c.env && Object.keys(c.env).length > 0 ? Object.entries(c.env).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)) : null;
    const key = JSON.stringify([c.run, cwdOf(root, c), env]);
    const g = seen.get(key);
    if (g === undefined) {
      seen.set(key, order.length);
      order.push([i]);
    } else {
      (order[g] as number[]).push(i);
    }
  });
  return order;
}

/** The directory a check's command runs in: its `cwd` under the root, or the root. */
function cwdOf(root: string, c: Check): string {
  return c.cwd === undefined ? root : join(root, c.cwd);
}

/** A unit of a group that runs: whether a change reached it, and the footprint its run is recorded with. */
interface Due {
  unit: UnitDef;
  rests: Rests;
  reached: boolean;
  footprint: Footprint;
}

/**
 * Judges the checks of one group of `groups`, in its order. A check with `file` is judged alone. A unit
 * whose last green still holds is kept, unless `units` says every check runs. The group's command runs
 * once for every other check, unless none is left. The store opens into `opened` the first time a
 * unit needs it. The id of the job the command runs as goes to `onJob` as soon as the job starts.
 */
async function runGroup(
  root: string,
  members: Check[],
  logDir: string,
  units: Units,
  project: Project,
  opened: { store?: Store },
  onJob: (job: string) => void,
): Promise<Verdict[]> {
  const first = members[0];
  if (first === undefined) return [];
  if (first.run.length === 0) return [verdict(first, judgeFile(root, first), null)];
  let tree: Tree | undefined;
  const verdicts: (Verdict | undefined)[] = members.map(() => undefined);
  const judged: [number, Due | null][] = [];
  members.forEach((c, i) => {
    const r = rests(c);
    if (r === "whole") {
      judged.push([i, null]);
      return;
    }
    opened.store ??= Store.open(units.store);
    tree ??= Tree.list(root);
    const unit = unitOf(root, c, project, r);
    const reached = compare(opened.store, tree, [unit]).run.length > 0;
    if (reached || units.all) judged.push([i, { unit, rests: r, reached, footprint: footprintOf() }]);
    else verdicts[i] = { ...verdict(c, "", null), kept: true };
  });
  const lead = judged[0];
  if (lead !== undefined) {
    const ids = judged.map(([i]) => (members[i] as Check).id);
    const secs = Math.max(...judged.map(([i]) => (members[i] as Check).timeout ?? project.checks.timeout));
    tree = await footprints(root, members, judged, tree, project, secs);
    const c0 = members[lead[0]] as Check;
    const ran = await runCommand({
      root,
      argv: c0.run,
      cwd: cwdOf(root, c0),
      env: envOf(project, c0),
      secs,
      kind: "check",
      hide: project.checks.hide_windows,
      onWait: toStderr,
      onStart: onJob,
    });
    for (const [i, due] of judged) {
      const c = members[i] as Check;
      const log = writeLog(logDir, c, ran, ids.filter((id) => id !== c.id));
      const v = verdict(c, whyRan(c, ran, secs), log);
      if (due !== null && opened.store !== undefined && tree !== undefined) {
        const isVoid = unitRecord(opened.store, tree, due, v);
        if (!v.green && !due.reached && !isVoid) {
          opened.store.recordMiss(due.unit.id);
          v.missed = true;
        }
      }
      verdicts[i] = v;
    }
  }
  return verdicts.filter((v) => v !== undefined);
}

/** What a command check's last green rests on, which says whether it can be kept. */
export type Rests = "declared" | "cargo" | "whole";

/** The target selectors of `cargo test`. With none, cargo runs the doctests too, which no binary holds. */
const CARGO_TARGETS = ["--test", "--tests", "--bin", "--bins", "--lib", "--example", "--examples", "--bench", "--benches", "--all-targets"];

/** What the check `c` rests on. A `cargo test` that runs from the root and names its targets rests on its binaries. */
export function rests(c: Check): Rests {
  if (c.run.length === 0) return "whole";
  if (c.cwd === undefined && observable(c.run)) return "cargo";
  return c.inputs !== undefined && c.inputs.length > 0 ? "declared" : "whole";
}

/** Whether the cargo observer sees all the `cargo test` argv `argv` runs: before its `--` it names a target selector, and no `--doc`, `--manifest-path` or `-Z` flag. */
function observable(argv: string[]): boolean {
  if (!isCargoTest(argv)) return false;
  let selected = false;
  for (const arg of splitCargo(argv)[0]) {
    const flag = arg.split("=")[0] as string;
    if (flag === "--doc" || flag === "--manifest-path" || flag.startsWith("-Z")) return false;
    selected ||= CARGO_TARGETS.includes(flag);
  }
  return selected;
}

function unitId(check: string): string {
  return `check:${check}`;
}

/** The unit of the check `c`, which rests as `r` says. */
function unitOf(root: string, c: Check, project: Project, r: Rests): UnitDef {
  // The loop's variables change every session, so they stay out of the definition.
  let def = `${JSON.stringify(c)}\n${root}\n${project.checks.timeout} ${JSON.stringify(project.checks.env)}`;
  if (r === "cargo") def += "\nobserved: cargo v1";
  return unitDef(unitId(c.id), def);
}

/**
 * Gives each unit of `judged` its footprint and primes the tree with it, before the group's command
 * runs, and gives the tree. A declared unit rests on its `inputs`. A cargo unit rests on what its
 * binaries read, and on its `inputs` too, and lists and primes the tree whole before the build.
 */
async function footprints(root: string, members: Check[], judged: [number, Due | null][], tree: Tree | undefined, project: Project, secs: number): Promise<Tree | undefined> {
  const cargoLead = judged.find(([, d]) => d?.rests === "cargo");
  if (cargoLead !== undefined) {
    tree = Tree.list(root);
    tree.prime(wholeTree());
  }
  if (tree === undefined) return tree;
  for (const [i, due] of judged) {
    const inputs = (members[i] as Check).inputs ?? [];
    if (due !== null && (due.rests === "declared" || inputs.length > 0)) {
      due.footprint = declared(tree, inputs);
      tree.prime(due.footprint);
    }
  }
  if (cargoLead === undefined) return tree;
  const seen = await observe(root, members[cargoLead[0]] as Check, project, secs, tree);
  for (const [, due] of judged) if (due?.rests === "cargo") union(due.footprint, seen);
  return tree;
}

/** What the test binaries of the cargo check `c` read in `tree`. A build that fails or lists no binary rests on the whole tree. */
async function observe(root: string, c: Check, project: Project, secs: number, tree: Tree): Promise<Footprint> {
  const built = await build(root, { ...project.checks, timeout: secs }, envOf(project, c), splitCargo(c.run)[0], toStderr);
  if (typeof built.build === "string" || built.build.tests.length === 0) return wholeTree();
  const seen = footprintOf();
  for (const t of built.build.tests) {
    const f = built.build.footprint(t, tree, root);
    union(seen, isEmpty(f) ? wholeTree() : f);
  }
  return seen;
}

/** Records how the run of `due` ended, as `v` judged it. A green run whose inputs changed while it ran turns red. Says whether it was void. */
function unitRecord(store: Store, tree: Tree, due: Due, v: Verdict): boolean {
  const recorded = store.record(tree, due.unit, v.green ? "green" : "red", due.footprint);
  if (recorded.kind !== "void") return false;
  v.green = false;
  v.why = `${recorded.changed.join(", ")} changed while it ran, so its result is void`;
  return true;
}

/** The verdict of `c`: green when `why` is empty. */
function verdict(c: Check, why: string, log: string | null): Verdict {
  return { id: c.id, green: why === "", why, log, kept: false, missed: false };
}

/** The variables a check's process sees: the kit's, then `[checks] env`, then the loop's variables, then the check's own `env`. */
export function envOf(project: Project, c: Check): Record<string, string | undefined> {
  const env = checkEnv(project.checks);
  for (const over of [project.vars, c.env ?? {}]) {
    for (const [k, v] of Object.entries(over)) {
      for (const name of Object.keys(env).filter((n) => n.toUpperCase() === k.toUpperCase())) delete env[name];
      if (v !== undefined) env[k] = v;
    }
  }
  return env;
}

function toStderr(line: string): void {
  process.stderr.write(`${line}\n`);
}

/** Why the check `c` is red on what its command did, run with a timeout of `secs`; empty when green. */
function whyRan(c: Check, ran: Ran, secs: number): string {
  return ran.ended === "timed out" ? `timed out after ${secs}s` : judge(c, ran.code, ran.stdout, ran.stderr);
}

/** Writes what a command did as the log of `c`, `<log dir>/<id>.log`, naming `others`, the other checks the same run judged. */
function writeLog(logDir: string, c: Check, ran: Ran, others: string[]): string {
  mkdirSync(logDir, { recursive: true });
  const log = join(logDir, `${c.id}.log`);
  const shared = others.length === 0 ? "" : `this run also judged: ${others.join(", ")}\n`;
  const code = ran.code === null ? "none" : String(ran.code);
  writeFileSync(log, `$ ${c.run.join(" ")}\nexit ${code}${note(ran)}\n${shared}--- stdout ---\n${ran.stdout}\n--- stderr ---\n${ran.stderr}\n`);
  return log;
}

/** Runs one check alone and judges it, as a group of one. Its command holds a heavy-job slot. */
export async function runCheck(root: string, c: Check, logDir: string, project: Project, kind: Kind = "check"): Promise<Verdict> {
  if (c.run.length === 0) return verdict(c, judgeFile(root, c), null);
  const secs = c.timeout ?? project.checks.timeout;
  const ran = await runCommand({ root, argv: c.run, cwd: cwdOf(root, c), env: envOf(project, c), secs, kind, hide: project.checks.hide_windows, onWait: toStderr });
  return verdict(c, whyRan(c, ran, secs), writeLog(logDir, c, ran, []));
}

/** How a command's run ended: it exited, a process it started held its output open past `LINGER_S`, or it ran past its timeout. */
export type Ended = "exited" | "held" | "timed out";

/** What one command did. Its stdout and stderr each keep their first and last `OUTPUT_HALF` bytes. */
export interface Ran {
  /** Its exit code, or null when it timed out or could not start. */
  code: number | null;
  stdout: string;
  stderr: string;
  ended: Ended;
}

/** How a run ended, as a clause that follows its exit code; empty for a plain exit. */
export function note(ran: Ran): string {
  if (ran.ended === "timed out") return " (timed out)";
  if (ran.ended === "held") return ` (a process it started held its output open ${LINGER_S}s after it exited, and was killed)`;
  return "";
}

/** One command for `runCommand`. */
export interface CommandRun {
  /** The project root, whose job store records the run. */
  root: string;
  argv: string[];
  cwd: string;
  env: Record<string, string | undefined>;
  /** Its timeout, in seconds. */
  secs: number;
  kind: Kind;
  hide: boolean;
  /** With it, the run holds a heavy-job slot, and a wait for one is said here. */
  onWait: ((line: string) => void) | null;
  /** The command's stdin. `NUL` when absent. */
  stdin?: string;
  /** Gets the id of the command's job as soon as the job starts. */
  onStart?: (job: string) => void;
}

/**
 * Runs one command as a job of `cmd.kind`, killing its tree at its timeout, and keeps its stdout and
 * stderr apart. Once the command exits, its output gets `LINGER_S` to close. A command that cannot
 * start ends with no code and says why on its stderr.
 */
export async function runCommand(cmd: CommandRun): Promise<Ran> {
  const command = cmd.argv.join(" ");
  const lease = cmd.onWait === null ? null : await slot(cmd, command, cmd.onWait);
  try {
    const out = new Bounded();
    const err = new Bounded();
    const env = lease === null ? cmd.env : { ...cmd.env, [SLOT_VAR]: lease.id };
    let job: ReturnType<typeof startJob>;
    try {
      job = startJob(storeOf(cmd.root), {
        kind: cmd.kind,
        argv: cmd.argv,
        cwd: cmd.cwd,
        env,
        command,
        project: cmd.root,
        onOutput: (chunk) => out.push(chunk),
        onError: (chunk) => err.push(chunk),
        lingerMs: LINGER_S * 1000,
        hide: cmd.hide,
        stdin: cmd.stdin,
      });
    } catch (e) {
      return { code: null, stdout: "", stderr: `cannot run \`${cmd.argv[0] ?? ""}\`: ${(e as Error).message}`, ended: "exited" };
    }
    cmd.onStart?.(job.id);
    await lease?.hold(job.id);
    let timedOut = false;
    let cut = false;
    const timer = setTimeout(() => {
      // A timeout while the output lingers ends a command that has exited, so the output was held.
      if (job.tree.exitCode() === null) timedOut = true;
      else cut = true;
      try {
        job.tree.kill(KILLED_CODE);
      } catch {}
    }, cmd.secs * 1000);
    const code = await job.done.finally(() => clearTimeout(timer));
    const ended: Ended = timedOut ? "timed out" : job.held || cut ? "held" : "exited";
    return { code: timedOut ? null : code, stdout: out.text(), stderr: err.text(), ended };
  } finally {
    await lease?.release();
  }
}

/** Takes one heavy-job slot for `command`, unless this process runs under a holder. */
async function slot(cmd: CommandRun, command: string, onWait: (line: string) => void): Promise<Lease | null> {
  if ((cmd.env[SLOT_VAR] ?? "") !== "") return null;
  const asked = await takeSlots(looperHome(cmd.env), "one", { kind: cmd.kind, project: cmd.root, command }, onWait);
  return "lease" in asked ? asked.lease : null;
}

/** Why a command check that ran to its end is red, or an empty string when it is green. */
export function judge(c: Check, code: number | null, stdout: string, stderr: string): string {
  const shown = code === null ? "without a code" : String(code);
  if ((c.exit ?? "zero") === "zero") {
    if (code !== 0) return `exited ${shown}, want 0`;
  } else if (code === null || code === 0) {
    return `exited ${shown}, want nonzero`;
  }
  for (const s of c.stdout_contains ?? []) if (!stdout.includes(s)) return `stdout lacks ${quoted(s)}`;
  for (const s of c.stderr_contains ?? []) if (!stderr.includes(s)) return `stderr lacks ${quoted(s)}`;
  if (c.stdout_exact !== undefined && stdout.trimEnd() !== c.stdout_exact.trimEnd()) return "stdout is not exactly what the check wants";
  if (!orderHolds(stdout, c.stdout_ordered ?? [])) return `stdout lacks ${quoted(missing(stdout, c.stdout_ordered))} in order`;
  if (hasOutputMatcher(c)) {
    const why = judgeText(c, "output", `${stdout}${stderr}`);
    if (why !== "") return why;
  }
  for (const t of c.tests ?? []) if (!testPassed(t, stdout) && !testPassed(t, stderr)) return `test \`${t}\` did not run and pass`;
  return "";
}

/** `s` as Rust's `{:?}` writes a string, which a reason quotes. */
function quoted(s: string): string {
  return JSON.stringify(s);
}

/** The first of `ordered` that `text` lacks after the ones before it, or empty when it has them all. */
function missing(text: string, ordered: string[] | undefined): string {
  let at = 0;
  for (const s of ordered ?? []) {
    const i = text.indexOf(s, at);
    if (i < 0) return s;
    at = i + s.length;
  }
  return "";
}

/** Whether `text` holds each of `ordered`, each after the one before it. */
function orderHolds(text: string, ordered: string[]): boolean {
  return missing(text, ordered) === "";
}

function hasOutputMatcher(c: Check): boolean {
  return c.output_contains !== undefined || c.output_exact !== undefined || c.output_ordered !== undefined;
}

/** Why `text`, which `what` names, fails the check's `output_*` matchers, or an empty string. */
function judgeText(c: Check, what: string, text: string): string {
  for (const s of c.output_contains ?? []) if (!text.includes(s)) return `${what} lacks ${quoted(s)}`;
  if (c.output_exact !== undefined && text.trimEnd() !== c.output_exact.trimEnd()) return `${what} is not exactly what the check wants`;
  if (!orderHolds(text, c.output_ordered ?? [])) return `${what} lacks ${quoted(missing(text, c.output_ordered))} in order`;
  return "";
}

/** Why a check with `file` and no `run` is red: the file is missing or short, or its text fails the `output_*` matchers. Empty when green. */
export function judgeFile(root: string, c: Check): string {
  const file = c.file;
  if (file === undefined) return "the check has neither `run` nor `file`";
  const path = join(root, file);
  let size: number;
  try {
    size = statSync(path).size;
  } catch {
    return `${file} does not exist`;
  }
  const min = c.min_bytes ?? 0;
  if (size < min) return `${file} has ${size} bytes, want at least ${min}`;
  if (!hasOutputMatcher(c)) return "";
  const kept = new Bounded();
  try {
    const fd = openSync(path, "r");
    try {
      const buf = Buffer.alloc(READ_CHUNK);
      for (let n = readSync(fd, buf); n > 0; n = readSync(fd, buf)) kept.push(buf.subarray(0, n));
    } finally {
      closeSync(fd);
    }
  } catch (e) {
    return `cannot read ${file}: ${(e as Error).message}`;
  }
  return judgeText(c, file, kept.text());
}

/**
 * Whether output shows the named test as run and passed: a libtest line, or bun's or vitest's reporter.
 * A line is read without its ANSI colour codes.
 */
export function testPassed(name: string, out: string): boolean {
  return out.split("\n").some((raw) => {
    const line = plain(raw).trimEnd();
    if (line.startsWith("test ") && line.endsWith(" ... ok")) {
      const path = line.slice("test ".length, -" ... ok".length);
      if (path === name || path.endsWith(`::${name}`)) return true;
    }
    const t = line.trimStart();
    return (t.startsWith("(pass)") || t.startsWith("✓")) && hasWord(t, name);
  });
}

/** `line` without its ANSI CSI sequences: an escape, `[`, then bytes up to one in `@`..`~`. */
function plain(line: string): string {
  return line.replace(/\x1b\[[\x20-\x3f]*[\x40-\x7e]?/g, "");
}

function hasWord(hay: string, word: string): boolean {
  const isWord = (ch: string | undefined) => ch !== undefined && /[\p{L}\p{N}_]/u.test(ch);
  for (let i = hay.indexOf(word); i >= 0; i = hay.indexOf(word, i + 1)) {
    if (!isWord(hay[i - 1]) && !isWord(hay[i + word.length])) return true;
  }
  return false;
}
