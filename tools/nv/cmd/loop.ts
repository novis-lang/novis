// `bun nv loop --list`: the live goal's acceptance plan. One line per check, `[<stage number> <stage
// title>]`, its kind, its name and what it runs, then `list: N check(s) match` last. The live goal is the one
// `data/chain.json` names `live`, and its plan is `lib/chain.ts`'s `goalPlan`: the goal's own record, which
// is the whole plan the driver runs.
//
// Three filters narrow it, and a check is printed when it matches every one given:
//   - `--stage <label>` — the stage's number, or the start of its label `8 the driver` or of its title;
//   - `--name <text>` — text inside the check's name, case-insensitive;
//   - `--feature <id>` — a proofs check over that feature: its name is `proofs: <id>`,
//     with or without a `(1/3)` part, or its `argv` passes `<id>` to `--group` or `--id`.
// A check with no name of its own is named by its `file`, so `--name` reaches a fixture's check too.
//
// `bun nv loop --goal` prints the goal table `[g]` prints, one row per stage, without a run. Its results
// are the checks whose atoms the selection store last recorded green (`driver/runner.ts` `lastGreen`),
// as the status row's are, and its session number is `.loop/run.json`'s `index`. No session runs under
// this command, so the table's last
// line names none. The table is as wide as the terminal, or 100
// columns when the output is not one, and it draws its lines in box-drawing characters on Windows and
// under a UTF-8 locale.
//
// `bun nv loop --run` takes the same three filters, at least one of them, and runs each check they
// select once, every atom of it, through `driver/runner.ts`'s `PlanSweep`, recording each run. It prints
// one line per check, `ok`, `FAIL` with the
// line the ledger would quote, or `SHORT` for a suite below its `minPassing`, then `run: N green, M red`.
// What each process is doing goes to stderr as it starts. A session proves its own check this way, one
// command at a time, rather than by starting a sweep. It and every sweep below hold `driver/origin.ts`'s
// listener up while they run a check that talks to it, as the fixture of `examples/http.nvs` does. A sweep
// below that uses the origin, a Linux leg or a perf guard first takes `driver/sweep-lock.ts`'s lock, so one
// tree on this machine uses them at a time; a sweep that uses none of them, and `--run`, do not take it.
//
// `bun nv loop --goal-only` is the acceptance sweep, `driver/accept.ts`'s `acceptance` over
// `driver/runner.ts`'s `PlanSweep`: the change since the selection store's tree is read once, and a
// check is started only when the change reaches one of its atoms (`select/checks.ts`), each check in the
// tiers' order, with a stop at the first red. A check the change does not reach prints nothing, and the
// sweep says how many there were. It ends on `NOT GREEN: <the red check's line>` or `GOAL REACHED`.
// `--full` reaches every atom of every check, and `--collect` runs past every red and names each one
// after the first on an `also red:` line. `--gate-shut` is a turn's sweep in nine of ten: the heavy
// checks and the Linux legs are held, a held check the change reached is owed, and a green sweep ends
// on `GREEN` rather than `GOAL REACHED`. The three filters narrow it to the checks they select, and a
// narrowed sweep that passes ends on `GREEN` instead, since it has not asked the whole plan. Every sweep
// records what it ran, and moves the store's tree past the change, green or red.
//
// `bun nv loop --owed` names the carried checks, the stages whose label says `floor`, that the change
// since the store's tree reaches, and runs nothing: a check whose atom is new, red or owed, or holds a key
// the change moved. A change made outside a run reaches the checks that read what it touched, and
// `tools/git-hooks/pre-push` refuses a push while this names one. The goal's own checks and a
// `memoize = false` check are never owed. `bun nv loop --settle` is the sweep over the carried checks
// alone, every red collected, so it runs what `--owed` names and ends on `SETTLED` or `NOT GREEN`.
//
// `bun nv loop` with none of those modes is the run. Started by hand, with no `NOVIS_LOOP_RUN`, it is
// `driver/respawn.ts`: it names the run and starts one turn after another, each its own process. A turn
// is `bun nv loop` with that variable set: one session, its acceptance sweep and its ledger lines in
// `.loop/log.md`, then exit 75, which asks for the next turn. A session that changed driver code the turn
// had imported is not swept by that turn, which would judge it with the code it replaced: the turn exits
// 75 at once, and the next one serves no session and sweeps that one (`driver/launch.ts`'s
// `driverChanged`). It takes `--model` (`opus`), `--effort`, `--permission-mode` (`auto`),
// `--max-sessions`, `--max-stalls` (10), `--max-retries` (3), `--max-limit-wait` (21600 seconds),
// `--min-free-gb` (`nv disk`'s `MIN_FREE_GB`), `--keep-runs` (`nv disk`'s `KEEP_RUNS`) and `--no-hold`. The
// run is the one `NOVIS_LOOP_RUN` names, and `.loop/run.json` carries its counts from turn to turn. A turn
// refuses a tree whose `.loop/running` names another run, and a turn that ends the run removes the marker
// however it ends.
//
// The first turn of a run refuses a disk with less than `--min-free-gb` free, then preflights the live
// goal's Docker daemon and brings up its `env.docker` services (`driver/gates.ts`).
//
// `--side <slug>`, ahead of every other argument, puts any mode above on side goal `slug` instead of the
// live goal: it sets `NOVIS_SIDE_GOAL`, which `nv orient`, `nv session` and every turn and session of the run
// inherit, and the plan is `lib/chain.ts`'s `sidePlan`. The run it starts outside the side goal's worktree
// is `driver/side.ts`'s launcher, which readies the worktree and runs the worktree's own driver there. A side
// run is the chain run with one difference: a green sweep with both goal-end gates green ends the run on
// `SIDE GOAL GREEN` and the landing steps, and nothing is switched.
//
// The console is `driver/console.ts`'s: the session's transcript through `driver/transcript.ts`, the
// driver's stamped steps, and the live block, whose status line is `driver/status.ts`'s one-row
// `statusRow`, with the `g`, `r`, `s`, `p`, `h` and `i` keys under it. `--max-result-lines` (60),
// `--max-input-lines` (40) and `--max-line-chars` (500) cap the transcript, `--full-output` removes the
// caps, and `--no-status` paints no live block. A Ctrl-C ends the session, sweeps what it left into a wip
// commit and ends the run.
//
// The session is `driver/launch.ts`'s: the session prompt with `nv orient`'s pack behind it, down
// `claude -p`'s stdin as stream-json, or down the stand-in `NOVIS_LOOP_CLAUDE` names, a command a
// rehearsal uses to play a session on a scratch tree. A session that ends without wrapping has its own
// uncommitted paths swept into a wip commit. A session the usage wall refused, the API refused as
// overloaded, or the CLI failed is swept and run again inside the same turn, and one whose stream dropped
// is rejoined with `claude --resume`: `driver/sweep.ts` says how and how often.
//
// The acceptance sweep behind it is every check in the plan, carried or the goal's own, and what starts
// is what the session's change reached. The heavy checks (`accept.ts`'s `heldByGate`: the release
// profile, fuzz, TSan, the database matrix and the checks never memoized, but never a setup command,
// which the fixtures behind it need) and the Linux legs are held in nine turns of ten (`FLOOR_GATE_EVERY`, counted in the store as `floor-gate:since`), and a heavy check a held
// sweep reached is owed until the gate opens. On the turn the gate opens by its count, the full run
// (`select/full.ts`, `bun nv select --full`) comes first: every atom that is not heavy runs and records,
// and a selection miss it finds is a red of the sweep. A scoped sweep that is green with checks held runs again over the whole
// plan, collecting every red, since a goal is never reached on a held check. That goal-end sweep runs the
// heavy checks and legs its goal reached, and on one goal end in three (`HEAVY_FULL_EVERY`, counted in the
// store as `heavy-full:since`) every one of them, the safety net for the keys their twins record. After every sweep the disk is swept (`nv disk --clean`'s policy). A sweep that
// would reach the goal also runs the goal-end gates, rustdoc and owner, and a goal is reached only when
// the sweep and both gates are green. Then `advance` makes the next goal on the chain live, commits
// `data/chain.json`, brings up its services, and the run goes on; after the last goal the chain is complete.
//
// A DONE the sweep refuses gets a retry session, `DONE_RETRIES` per goal. A turn that does not simply go on
// ends in a verdict, and `afterTurn` settles it: a repairable one gets a repair session with
// `docs/agent/repair-prompt.md` and the verdict, `REPAIRS_PER_GOAL` per goal; one that needs a person holds
// the run until `p` or deleting `.loop/pause` lifts it, or ends the run under `--no-hold`; the same verdict
// twice across a hold ends it. `HOLD_KINDS` and `REPAIR_KINDS` say which is which.
//
// `--list` and `--goal` exit 0 when the plan is read, whether or not a check matches. `--run`,
// `--goal-only` and `--settle` exit 0 when every check they reached is green and 1 when one is not, and
// `--owed` exits 1 while it names a check. Each exits 2 on a
// bad argument or when there is no live goal to read. A turn exits 75 when the run goes on and 0 when it
// ends.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { KEEP_RUNS, MIN_FREE_GB } from "./disk.ts";
import { head } from "../lib/git.ts";
import { run as runProc } from "../lib/proc.ts";
import { AGAIN, LOGDIR, RUN_ENV, RUNDIR, driverChanged, driverFiles, launch, ledger, loadRun, openingLine, pendingJudge, saveRun, type LaunchOptions, type RunState } from "../driver/launch.ts";
import { respawn } from "../driver/respawn.ts";
import { AllowWatch, loadSeen, record as recordAllow, saveSeen, settle as settleAllow } from "../driver/allow.ts";
import { insideWorktree, landing, launchSide } from "../driver/side.ts";
import { bringUp, docGate, enoughDisk, ownerGate, preflight, sweepDisk } from "../driver/gates.ts";
import { chainGoals, type Goal, goalPlan, leaveGoal, liveGoal, SIDE_ENV, sideGoal, sidePlan } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { ENV as PROGRESS_ENV, readProgress } from "../lib/progress.ts";
import { goal as goalType } from "../schema/goal.ts";
import { goalTable, Session, statusRow, type Results } from "../driver/status.ts";
import {
  LIMIT,
  MAX_RESUMES,
  MAX_WALLS,
  OVERLOAD_BACKOFF,
  OVERLOAD_STATUS,
  RESUME_BACKOFF,
  RESUME_PROMPT,
  type RateLimit,
  type Swept,
  Touched,
  apiErrorStatus,
  backoff,
  markInterrupted,
  nthWait,
  readLimit,
  rememberLimit,
  standingLimit,
  streamDropped,
  wallAfter,
} from "../driver/sweep.ts";
import { C, CONSOLE, CONTROL, HALT, clock, LiveSession, PAUSE, RETRY, SAY, SLICES, STOP, TICKER, VERIFY, consoleMode, hms, holdPause, mmss, say, step, verdict, wait } from "../driver/console.ts";
import { Tree as ProcTree } from "../driver/proctree.ts";
import { type Caps, DEFAULT_CAPS, Renderer } from "../driver/transcript.ts";
import { ENV as WRITES_ENV } from "../lib/written.ts";
import { holdOrigin, type Origin } from "../driver/origin.ts";
import { needLine, sharedResources, takeSweepLock } from "../driver/sweep-lock.ts";
import { type AcceptanceResult, type Check, PROGRAM_KINDS, acceptance, allReds, heldByGate, isCarried, owedChecks, tiers, withSetups } from "../driver/accept.ts";
import { type LegsOptions, legSteps, linuxLegs, startWslBuild } from "../driver/legs.ts";
import { lastGreen, PlanSweep } from "../driver/runner.ts";
import { writeGoalPlan } from "../renderers/goal-plan.ts";
import { SelectStore } from "../select/store.ts";
import { describeFull, fullRun } from "../select/full.ts";

export const summary = "the loop driver: one turn with no mode, or the live goal's plan: nv loop [--side <slug>] --list|--run|--goal-only [--full] [--collect] [--gate-shut] [--stage <label>] [--name <text>] [--feature <id>] | --goal | --owed | --settle";

const RUN = ".loop/run.json";

const USAGE =
  "bun nv loop [--side <slug>] [--model <m>] [--effort <e>] [--permission-mode <p>] [--max-sessions <n>] [--max-stalls <n>] [--max-retries <n>] [--max-limit-wait <seconds>] [--max-result-lines <n>] [--max-input-lines <n>] [--max-line-chars <n>] [--full-output] [--no-status] [--no-hold] [--min-free-gb <n>] [--keep-runs <n>] | --list|--run|--goal-only [--full] [--collect] [--gate-shut] [--stage <label>] [--name <text>] [--feature <id>] | --goal | --owed | --settle";

/** The session prompt every turn's session opens with. */
const PROMPT = "docs/agent/session-prompt.md";
/** Turns since the sweep last ran the heavy checks and the Linux legs, in the selection store's `meta`. */
const FLOOR_META = "floor-gate:since";
/**
 * One sweep in this many runs the heavy checks and the Linux legs. A regression only they see waits at
 * most this many sessions to be named; every other check runs in the sweep after the change that reached it.
 */
const FLOOR_GATE_EVERY = 10;
/** Goal ends since the last one that ran every heavy check and Linux leg, in the store's `meta`. */
const HEAVY_FULL_META = "heavy-full:since";
/**
 * One goal end in this many runs every heavy check and both Linux legs, whatever changed: the safety net
 * for the keys their twins record. Every other goal end runs the ones its change reached, so a heavy
 * check the keys missed waits at most this many goals to be named.
 */
const HEAVY_FULL_EVERY = 3;

interface Filters {
  stage?: string;
  name?: string;
  feature?: string;
  full?: boolean;
  collect?: boolean;
  /** `--gate-shut`: hold the heavy checks and the legs, as a turn does in nine of ten. */
  shut?: boolean;
}

function parse(args: string[]): Filters | null {
  const out: Filters = {};
  for (let i = 1; i < args.length; i += 2) {
    const flag = args[i]!;
    if (args[0] === "--goal-only" && (flag === "--full" || flag === "--collect" || flag === "--gate-shut")) {
      out[flag === "--full" ? "full" : flag === "--collect" ? "collect" : "shut"] = true;
      i--;
      continue;
    }
    const value = args[i + 1];
    if (value === undefined) return null;
    if (flag === "--stage") out.stage = value;
    else if (flag === "--name") out.name = value;
    else if (flag === "--feature") out.feature = value;
    else return null;
  }
  return out;
}

/** The check's name as the driver prints it: its own, or its fixture's run line. */
function nameOf(c: Check): string {
  return c.name ?? `run ${[...(c.args ?? []), c.file ?? ""].join(" ")}`.trimEnd();
}

function stageMatches(label: string, number: number, want: string): boolean {
  // A bare number names that stage alone, so `1` never reaches stages 10 to 12 through the prefix.
  const w = want.trim().toLowerCase();
  if (/^\d+$/.test(w)) return w === String(number);
  const l = label.toLowerCase();
  return l.startsWith(w) || l.slice(String(number).length + 1).startsWith(w);
}

/** Whether `--feature <id>` shows the check: a proofs check named for the feature, or one whose `--group` or `--id` is it. */
export function featureMatches(c: Check, id: string): boolean {
  const named = /^proofs: (.+?)(?: \(\d+\/\d+\))?$/.exec(c.name ?? "");
  if (named?.[1] === id) return true;
  const argv = c.argv ?? [];
  return argv.some((a, i) => (a === "--group" || a === "--id") && argv[i + 1] === id);
}

/** One check as `--list` prints it: what a session reads to decide whether a `want` is producible. */
function line(c: Check, label: string): string {
  const head = `  [${label}] ${c.kind.padEnd(11)}`;
  if (c.file !== undefined) {
    // A check's own `args` go ahead of the fixture, which is the order the driver runs them in.
    return `${head} nvs ${["run", ...(c.args ?? []), c.file].join(" ")}`;
  }
  if (c.kind === "command") {
    const note = c.setup ? "  (setup: runs before the fixtures)" : c.overlap ? "  (overlap: runs beside the sweep)" : c.memoize ? "  (memoized against the tree)" : "";
    return `${head} ${nameOf(c)}: ${(c.argv ?? []).join(" ")} in ${c.cwd ?? "."}${note}`;
  }
  const driver = c.kind === "nvs-suite" ? "nvs" : "cargo";
  return `${head} ${nameOf(c)}: ${driver} ${(c.args ?? []).join(" ")}`;
}

/** The goal a run works on: the live goal and its place on the chain, or a side goal, whose place is null. */
interface Current {
  slug: string;
  num: number | null;
}

/** Where `live` stands, as the driver prints it: `12 of 179`, or `a side goal, off the chain`. */
function placeOf(live: Current, total: number): string {
  return live.num === null ? "a side goal, off the chain" : `${live.num} of ${total}`;
}

/**
 * The goal this process works on, its plan and the chain's length, or null once the reason it has none is
 * printed. It is the side goal `SIDE_ENV` names, with `sidePlan`'s plan, or else the live goal, with
 * `goalPlan`'s: the goal's own record.
 */
function livePlan(): { live: Current; goal: Goal; total: number } | null {
  const goals = chainGoals();
  const side = sideGoal();
  if (side !== null) {
    const goal = sidePlan(side);
    if (goal === null) {
      console.error(`nv loop: the side goal \`${side}\` has no record at data/goals/side/${side}.json`);
      return null;
    }
    return { live: { slug: side, num: null }, goal, total: goals.length };
  }
  if (process.env[SIDE_ENV]) {
    console.error(`nv loop: ${SIDE_ENV} names \`${process.env[SIDE_ENV]}\`, and there is no docs/agent/goals/side/<slug>.md by that name`);
    return null;
  }
  const live = liveGoal(goals);
  if (live === null) {
    console.error("nv loop: data/chain.json names no live goal on the chain, so there is no plan to read");
    return null;
  }
  const goal = goalPlan(live.slug);
  if (goal === null) {
    console.error(`nv loop: the live goal \`${live.slug}\` has no record at data/goals/${live.slug}.json`);
    return null;
  }
  return { live, goal, total: goals.length };
}

/** `path`'s JSON object, or an empty one when it is absent or not an object. */
function jsonObject(path: string): Record<string, unknown> {
  try {
    const v = JSON.parse(readFileSync(join(ROOT, path), "utf8"));
    return typeof v === "object" && v !== null && !Array.isArray(v) ? v : {};
  } catch {
    return {};
  }
}

async function goalView(): Promise<number> {
  const found = livePlan();
  if (found === null) return 2;
  const { live, goal, total } = found;
  const plan = { slug: live.slug, stages: goal.stages, checks: goal.checks as Check[] };
  const results: Results = await lastGreen(plan.checks);
  const session = new Session(plan, results);
  session.begin(Number(jsonObject(RUN).index ?? 0) || 0);
  const locale = process.env.LC_ALL || process.env.LC_CTYPE || process.env.LANG || "";
  const utf8 = process.platform === "win32" || /utf-?8/i.test(locale);
  const width = process.stdout.columns ?? 100;
  for (const row of goalTable({ plan, results, session, position: live.num, total, commits: [], width, utf8 })) console.log(row);
  return 0;
}

/** The live goal's checks that match every filter given, with its stage labels, or null once the reason is printed. */
function selected(filters: Filters): { live: Current; goal: Goal; total: number; shown: Check[]; labelOf: (n: number) => string } | null {
  const found = livePlan();
  if (found === null) return null;
  const { live, goal } = found;
  const labels = new Map(goal.stages.map((s) => [s.number, `${s.number} ${s.title}`]));
  const labelOf = (n: number) => labels.get(n) ?? String(n);
  if (filters.stage !== undefined && !goal.stages.some((s) => stageMatches(labelOf(s.number), s.number, filters.stage!))) {
    console.error(`nv loop: no stage of \`${live.slug}\` is \`${filters.stage}\`; its stages are ${[...labels.values()].map((l) => `\`${l}\``).join(", ")}`);
    return null;
  }
  const shown = (goal.checks as Check[]).filter(
    (c) =>
      (filters.stage === undefined || stageMatches(labelOf(c.stage), c.stage, filters.stage)) &&
      (filters.name === undefined || nameOf(c).toLowerCase().includes(filters.name.toLowerCase())) &&
      (filters.feature === undefined || featureMatches(c, filters.feature)),
  );
  return { ...found, shown, labelOf };
}

function list(filters: Filters): number {
  const found = selected(filters);
  if (found === null) return 2;
  const { live, goal, total, shown, labelOf } = found;
  const path = `data/goals/${live.num === null ? "side/" : ""}${live.slug}.json`;
  const checks = goal.checks as Check[];

  console.log(`${path}: ${checks.length} checks, ${goal.files.length} fixtures, goal \`${live.slug}\` (${placeOf(live, total)})`);
  for (const c of shown) {
    console.log(line(c, labelOf(c.stage)));
    // Named cases are the half of a suite check a session acts on: the ones not yet on disk are the worklist.
    const cases = c.cases ?? [];
    if (cases.length > 0) {
      const absent = cases.filter((p) => !existsSync(join(ROOT, p)));
      console.log(`      ${cases.length} named case(s), ${absent.length} not written yet`);
      for (const p of absent) console.log(`        - ${p}`);
    }
  }
  const narrowed = filters.stage !== undefined || filters.name !== undefined || filters.feature !== undefined;
  if (!narrowed) console.log(`  valgrind sweep over every fixture except: ${[...(goal.env.valgrind?.skip ?? [])].sort().join(", ") || "nothing"}`);
  console.log(`list: ${shown.length} ${shown.length === 1 ? "check matches" : "checks match"}`);
  return 0;
}

/** Runs the checks the filters select, once each, every atom of each, in the sweep's tiers, and prints
 * one verdict a line. Every run records, and the store's tree moves past the change as after a sweep. */
async function runChecks(filters: Filters): Promise<number> {
  if (filters.stage === undefined && filters.name === undefined && filters.feature === undefined) {
    console.error("nv loop: --run needs --stage, --name or --feature; the whole plan is the acceptance sweep's to run");
    return 2;
  }
  const found = selected(filters);
  if (found === null) return 2;
  const { goal, shown, labelOf } = found;
  const order = tiers(shown, labelOf).flatMap((t) => t.checks);
  const sweep = await PlanSweep.open(goal.checks as Check[], labelOf, { full: true, onRun: (what) => console.error(`  .. ${what}`), say: (l) => console.error(`  ${l}`) });
  sweep.batch(order);
  let red = 0;
  const origin = sharedResources(order, false, () => false).origin ? await holdOrigin() : null;
  if (origin !== null) console.error(`  ${origin.line}`);
  try {
    for (const c of order) {
      const v = await sweep.check(c);
      if (v.fail !== "") console.log(`  FAIL  ${v.fail}`);
      else if (v.short !== "") console.log(`  SHORT ${v.short}`);
      else console.log(`  ok    ${nameOf(c)} [${labelOf(c.stage)}]`);
      if (v.fail !== "" || v.short !== "") red++;
    }
  } finally {
    origin?.close();
    sweep.close();
  }
  console.log(`run: ${order.length - red} green, ${red} red, of ${order.length} ${order.length === 1 ? "check" : "checks"}`);
  return red === 0 ? 0 : 1;
}

/** How a sweep reports itself: each process as it starts, each check the change reaches as it starts, and any other line. The Linux legs' steps are counted before the sweep starts, then the legs say when they begin and each step they finish. */
interface Progress {
  run: (what: string) => void;
  trace: (c: Check) => void;
  note: (text: string) => void;
  count: (steps: number) => void;
  plan: (steps: number) => void;
  step: () => void;
}

/**
 * One acceptance sweep over `checks`, on the selection store: the change since its tree is read once,
 * only the checks it reaches start, every run records, and the store's tree moves past the change when
 * the sweep ends, green or red. `onDone` is called with each check's verdict once the sweep ends, which
 * is how a caller learns them without a second run.
 */
async function sweepOver(
  goal: Goal,
  checks: Check[],
  labelOf: (n: number) => string,
  o: { full: boolean; collect: boolean; legs?: boolean; gateOpen?: boolean; heavyAll?: boolean; onDone?: (c: Check, green: boolean) => void; progress?: Progress },
): Promise<{ result: AcceptanceResult; secs: number; owed: number }> {
  const started = Date.now();
  const p: Progress = o.progress ?? {
    run: (what) => console.error(`  .. ${what}`),
    trace: (c) => console.error(`  check ${nameOf(c)} [${labelOf(c.stage)}]`),
    note: (text) => console.error(`  ${text}`),
    count: () => {},
    plan: () => {},
    step: () => {},
  };
  const sweep = await PlanSweep.open(goal.checks as Check[], labelOf, { full: o.full, heavyAll: o.heavyAll === true, onRun: p.run, say: p.note });
  const reachedOf = new Map<string, boolean>();
  const reached = (c: Check) => {
    let r = reachedOf.get(c.id);
    if (r === undefined) reachedOf.set(c.id, (r = sweep.reached(c)));
    return r;
  };
  const n = checks.filter(reached).length;
  const c = sweep.change;
  const since = c.full ? (c.global ?? "everything") : `${c.changes.length} path(s) changed since the recorded tree`;
  p.note(`${since}; ${n} of ${checks.length} check(s) reached, the other ${checks.length - n} not started`);
  // The two Linux legs, over the fixtures and suites this sweep reaches. They start with the overlap tier,
  // beside the release builds, and the release tier measures only once they end (`acceptance`'s `beside`).
  const legs: LegsOptions = {
    programs: checks.filter((c) => PROGRAM_KINDS.has(c.kind)),
    suites: checks.filter((c) => c.kind === "nvs-suite"),
    // From the whole goal, not `checks`: a setup check is heavy, so a shut floor gate holds it back while
    // the leg still runs the fixtures that read what it writes.
    setups: (goal.checks as Check[]).filter((c) => c.setup === true),
    files: goal.files,
    valgrindSkip: goal.env.valgrind?.skip ?? [],
    wslTarget: goal.env.wsl?.targetDir ?? null,
    gateOpen: o.gateOpen ?? false,
    reached: (leg) => o.full || sweep.legReached(leg),
    onGreen: (leg) => sweep.legGreen(leg),
    onRed: (leg) => sweep.legRed(leg),
    label: labelOf,
    onRun: p.run,
    onPlan: p.plan,
    onStep: p.step,
  };
  // The legs run only with the floor gate open; a leg the change reached with the gate shut is owed.
  const runLegs = o.legs === true && o.gateOpen === true;
  // A sweep that uses the origin, a Linux leg or a perf guard takes the lock, which another tree's sweep
  // using one of them may hold. The time spent waiting for it is not this sweep's cost.
  const need = sharedResources(checks.filter(reached), runLegs, legs.reached);
  p.note(needLine(need));
  const asked = Date.now();
  const lock = need.lock ? await takeSweepLock({ note: p.note }) : null;
  const waited = Date.now() - asked;
  let result: AcceptanceResult;
  let owed = 0;
  let origin: Origin | null = null;
  let legsRun: Promise<string> | null = null;
  try {
    if (runLegs) {
      p.count(legSteps(legs));
      p.note("the Linux legs run every fixture and suite again on Linux: they keep the floor gate's cadence, and run when the change reached one of those or code only Linux compiles");
      startWslBuild(legs);
    }
    if (need.origin) {
      origin = await holdOrigin();
      p.note(origin.line);
    }
    result = await acceptance(checks, {
      label: labelOf,
      sweep,
      reached,
      judged: (x) => sweep.judged(x),
      collect: o.collect,
      trace: p.trace,
      ...(runLegs
        ? {
            beside: () => {
              const run = linuxLegs(legs);
              legsRun = run;
              return run.then(
                () => {},
                () => {},
              );
            },
          }
        : {}),
    });
    // A sweep that stopped at a red check reports that red alone, as it would have at any later tier; a
    // collecting sweep that returned before the overlap tier runs the legs now.
    if (runLegs && (result.fail === "" || o.collect)) {
      const red = await (legsRun ?? linuxLegs(legs));
      if (red) result = { ...result, fail: result.fail ? allReds([result.fail, red]) : red };
    }
  } finally {
    origin?.close();
    lock?.release();
    const closed = sweep.close();
    owed = closed.owed;
    const ran = Object.entries(closed.ran).map(([k, n]) => `${n} ${k}`).join(", ");
    p.note(`the sweep ran ${ran || "no atom"}; ${owed} atom(s) the change reached and nothing ran stay owed`);
    if (closed.diverged.size > 0) {
      p.note(`${closed.diverged.size} atom(s) ended differently on the recording run than on the judged run, and run every time until they agree:`);
      for (const [id, why] of [...closed.diverged].sort(([a], [b]) => (a < b ? -1 : 1))) p.note(`  diverged  ${id}: ${why}`);
    }
  }
  for (const x of checks) o.onDone?.(x, result.verdicts.get(x.id) ?? false);
  return { result, secs: Math.round((Date.now() - started - waited) / 1000), owed };
}

/** The acceptance sweep over the checks the filters select, or over the whole plan. */
async function goalOnly(filters: Filters): Promise<number> {
  const found = selected(filters);
  if (found === null) return 2;
  const { goal, labelOf } = found;
  const narrowed = filters.stage !== undefined || filters.name !== undefined || filters.feature !== undefined;
  // `--gate-shut` is a turn's sweep in nine of ten: the heavy checks and the legs are held, and owed. A
  // setup command is never held, and a filter that selects a fixture runs the setups with it.
  const kept = filters.shut ? found.shown.filter((c) => !heldByGate(c)) : found.shown;
  const held = found.shown.length - kept.length;
  const shown = withSetups(kept, goal.checks as Check[]);
  console.log(`running the acceptance sweep over ${shown.length} ${shown.length === 1 ? "check" : "checks"}${filters.full ? " (full: every atom)" : ""}${filters.collect ? " (collecting every red)" : ""}${filters.shut ? ` (floor gate shut: ${held} heavy check(s) and the Linux legs held)` : ""}`);
  // The whole plan is a sweep with the floor gate open, and it runs both legs; a narrowed one runs neither.
  const open = !narrowed && !filters.shut;
  const { result, secs } = await sweepOver(goal, shown, labelOf, { full: filters.full === true, collect: filters.collect === true, legs: open, gateOpen: open });
  console.log(`cost: ${secs}s, ${result.ran} run, ${result.answered} not reached by the change, of ${shown.length}`);
  if (result.fail !== "") {
    console.log(`NOT GREEN: ${result.fail}`);
    return 1;
  }
  if (narrowed) console.log("GREEN: every check the filters select passes");
  else if (filters.shut) console.log("GREEN: every check but the heavy ones passes; the floor gate has still to run those and the Linux legs");
  else console.log("GOAL REACHED: every acceptance check passes, the WSL leg and the valgrind sweep with them");
  return 0;
}

/** `--owed`: names the carried checks the change since the store's tree reaches, and runs nothing. */
async function owed(): Promise<number> {
  const found = selected({});
  if (found === null) return 2;
  const { goal, labelOf } = found;
  const carried = (goal.checks as Check[]).filter((c) => isCarried(labelOf(c.stage)));
  const sweep = await PlanSweep.open(goal.checks as Check[], labelOf, { full: false });
  let names: string[];
  try {
    names = owedChecks(carried, labelOf, (c) => sweep.reached(c)).map(nameOf);
  } finally {
    sweep.discard();
  }
  if (names.length === 0) {
    console.log("owed: nothing -- every carried check is green over this tree");
    return 0;
  }
  const more = names.length > 6 ? `, +${names.length - 6} more` : "";
  console.log(`owed: ${names.length} carried check(s) are not green over this tree: ${names.slice(0, 6).join(", ")}${more}`);
  console.log("      `bun nv loop --settle` runs them, and only them");
  return 1;
}

/** `--settle`: the carried checks alone, only the ones the change reaches started and every red collected, so it runs what `--owed` names. */
async function settle(): Promise<number> {
  const found = selected({});
  if (found === null) return 2;
  const { goal, labelOf } = found;
  const carried = (goal.checks as Check[]).filter((c) => isCarried(labelOf(c.stage)));
  console.log(`running the carried floor, ${carried.length} ${carried.length === 1 ? "check" : "checks"} (collecting every red)`);
  const { result, secs } = await sweepOver(goal, carried, labelOf, { full: false, collect: true });
  console.log(`cost: ${secs}s, ${result.ran} run, ${result.answered} not reached by the change, of ${carried.length}`);
  if (result.fail !== "") {
    console.log(`NOT GREEN: ${result.fail}`);
    return 1;
  }
  console.log("SETTLED: every carried check is green over this tree");
  return 0;
}

/**
 * The sweep over the plan's checks whose ids are in `ids`, every red collected: what `nv affected --run`
 * runs once it has chosen them. Exits 0 when every one is green, 1 when one is red, and 2 when there is
 * no plan.
 */
export async function sweepIds(ids: Set<string>): Promise<number> {
  const found = selected({});
  if (found === null) return 2;
  const { goal, labelOf } = found;
  const chosen = (goal.checks as Check[]).filter((c) => ids.has(c.id));
  if (chosen.length === 0) return 0;
  console.log(`running ${chosen.length} acceptance ${chosen.length === 1 ? "check" : "checks"} (collecting every red)`);
  const { result, secs } = await sweepOver(goal, chosen, labelOf, { full: false, collect: true });
  console.log(`cost: ${secs}s, ${result.ran} run, ${result.answered} not reached by the change, of ${chosen.length}`);
  if (result.fail !== "") {
    console.log(`NOT GREEN: ${result.fail}`);
    return 1;
  }
  console.log("GREEN: every acceptance check the change reaches passes");
  return 0;
}

interface TurnFlags extends LaunchOptions, Caps {
  maxSessions: number;
  maxStalls: number;
  /** Non-zero CLI exits in a row that end the run. */
  maxRetries: number;
  /** Seconds of usage wall the turn sleeps through; a wall further out ends the run. */
  maxLimitWait: number;
  /** Whether the live block is painted; it never is when stdout is not a terminal. */
  status: boolean;
  /** Whether a verdict that needs a person holds the run rather than ending it; `--no-hold` ends it. */
  hold: boolean;
  /** Free gigabytes below which a run refuses to start; 0 starts anyway. */
  minFreeGb: number;
  /** Runs whose session logs the disk sweep keeps. */
  keepRuns: number;
}

function parseTurn(args: string[]): TurnFlags | null {
  const out: TurnFlags = {
    model: "opus",
    permissionMode: "auto",
    maxSessions: Infinity,
    maxStalls: 10,
    maxRetries: 3,
    maxLimitWait: 6 * 3600,
    ...DEFAULT_CAPS,
    status: true,
    hold: true,
    minFreeGb: MIN_FREE_GB,
    keepRuns: KEEP_RUNS,
  };
  for (let i = 0; i < args.length; i += 2) {
    if (args[i] === "--full-output" || args[i] === "--no-status" || args[i] === "--no-hold") {
      if (args[i] === "--full-output") out.maxResultLines = out.maxInputLines = out.maxLineChars = 0;
      else if (args[i] === "--no-status") out.status = false;
      else out.hold = false;
      i--;
      continue;
    }
    const value = args[i + 1];
    if (value === undefined) return null;
    const count = Number(value);
    if (args[i] === "--model") out.model = value;
    else if (args[i] === "--effort") out.effort = value;
    else if (args[i] === "--permission-mode") out.permissionMode = value;
    else if (args[i] === "--max-sessions" && Number.isInteger(count) && count > 0) out.maxSessions = count;
    else if (args[i] === "--max-stalls" && Number.isInteger(count) && count > 0) out.maxStalls = count;
    else if (args[i] === "--max-retries" && Number.isInteger(count) && count > 0) out.maxRetries = count;
    else if (args[i] === "--max-limit-wait" && Number.isFinite(count) && count >= 0) out.maxLimitWait = count;
    else if (args[i] === "--max-result-lines" && Number.isInteger(count) && count >= 0) out.maxResultLines = count;
    else if (args[i] === "--max-input-lines" && Number.isInteger(count) && count >= 0) out.maxInputLines = count;
    else if (args[i] === "--max-line-chars" && Number.isInteger(count) && count >= 0) out.maxLineChars = count;
    else if (args[i] === "--min-free-gb" && Number.isFinite(count) && count >= 0) out.minFreeGb = count;
    else if (args[i] === "--keep-runs" && Number.isInteger(count) && count > 0) out.keepRuns = count;
    else return null;
  }
  return out;
}

const MARKER = `${RUNDIR}/running`;
/** The environment variable that names a stand-in for the `claude` CLI; `standIn` reads it. */
const STAND_IN_ENV = "NOVIS_LOOP_CLAUDE";

/** The run `.loop/running` names, or "" when no run holds the tree. */
function holder(): string {
  try {
    return /^run:\s*(\S+)/m.exec(readFileSync(join(ROOT, MARKER), "utf8"))?.[1] ?? "";
  } catch {
    return "";
  }
}

/** Local time as the ledger writes it, `2026-09-24 22:45`. */
function stamp(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/**
 * The run ends: said once in the ledger and on the console, the marker removed, a hold taken at the
 * console dropped. `done` is a run that ended as asked, `--max-sessions`, rather than one that needs a
 * person. Returns the exit code.
 */
function finish(state: RunState, reason: string, done = false): number {
  ledger(`## run ended ${stamp()} -- ${reason}`);
  if (holder() === state.run) rmSync(join(ROOT, MARKER), { force: true });
  CONTROL.dropPause();
  saveRun(state);
  verdict(!done, done ? reason : `the run ended for good: ${reason}`);
  say("");
  say(`run done: ${state.served} session(s)`, C.CYAN);
  return 0;
}

/**
 * How a turn ended when the run does not simply go on. `budget`, `chain-complete` and `side-green` are a run
 * that ended as asked; `asked` is a stop someone pressed or wrote; the rest are verdicts a person may need to
 * answer.
 */
type Kind = "asked" | "budget" | "chain-complete" | "side-green" | "blocked" | "stalled" | "done-claim" | "cli-failed" | "chain-error" | "wall" | "wall-timeout";

interface Ended {
  kind: Kind;
  reason: string;
}

const end = (kind: Kind, reason: string): Ended => ({ kind, reason });

/**
 * The verdicts after which the run holds rather than ends: the goal is not finished, the tree is where the
 * session left it, and everything the next turn reads is read from disk, so a lifted hold starts a fresh
 * turn. `asked` is left out because someone said to end the run, and `chain-complete` has nothing left.
 */
const HOLD_KINDS = new Set<Kind>(["blocked", "stalled", "done-claim", "cli-failed", "chain-error", "wall"]);
/**
 * The held verdicts a session can answer without a person, so each first gets a repair session: the next
 * turn's session is given `REPAIR_PROMPT` and the verdict in place of the session prompt. `blocked` asks the
 * user for a decision, and `wall` is the usage limit, which nothing in the tree changes.
 */
const REPAIR_KINDS = new Set<Kind>(["stalled", "done-claim", "cli-failed", "chain-error"]);
/** Repair sessions one goal gets before a repairable verdict holds after all. */
const REPAIRS_PER_GOAL = 3;
/**
 * DONE claims one goal may have refused by its sweep and retried by a fresh session before a hand is asked.
 * A retry that fails on the very check it was handed holds at once: the same question asked twice.
 */
const DONE_RETRIES = 3;
const REPAIR_PROMPT = "docs/agent/repair-prompt.md";

/** What a run carries from turn to turn besides its counts, in `.loop/run.json`. */
interface Carried {
  /** DONE claims this goal has had retried. */
  done_retries: number;
  /** The check the last DONE retry was handed, or "". */
  retry_check: string;
  /** Repair sessions this goal has had. */
  repairs: number;
  /** The verdict the next session repairs, or "". */
  repair: string;
  /** The verdict the run last held on, as `[kind, reason]`, or empty. */
  last_hand: string[];
}

function carried(state: RunState): Carried {
  const s = state as RunState & Partial<Carried>;
  return {
    done_retries: Number(s.done_retries) || 0,
    retry_check: String(s.retry_check ?? ""),
    repairs: Number(s.repairs) || 0,
    repair: String(s.repair ?? ""),
    last_hand: Array.isArray(s.last_hand) ? s.last_hand.map(String) : [],
  };
}

function carry(state: RunState, c: Partial<Carried>): void {
  Object.assign(state, c);
}

/** The check a `goal check:` line names, its `name [stage]` label, so two sweeps red on one check compare equal. */
export function checkOf(fail: string): string {
  const at = fail.indexOf("]: ");
  return at < 0 ? fail.split("\n")[0]! : fail.slice(0, at + 1);
}

/**
 * What the run does with a turn that did not simply go on: a repairable verdict gets a repair session, a
 * verdict that needs a person holds the run until `p` or `.loop/pause` lifts it, and everything else ends it.
 * The same verdict twice across a hold ends the run rather than asking the identical question again.
 */
async function afterTurn(state: RunState, f: TurnFlags, out: Ended): Promise<number> {
  const { kind, reason } = out;
  if (kind === "budget" || kind === "chain-complete" || kind === "side-green") return finish(state, reason, true);
  const c = carried(state);
  if (REPAIR_KINDS.has(kind) && !c.repair && c.repairs < REPAIRS_PER_GOAL) {
    carry(state, { repairs: c.repairs + 1, repair: `${kind}: ${reason}` });
    const note = `not holding on ${kind} -- a repair session gets it first (${c.repairs + 1} of ${REPAIRS_PER_GOAL} on this goal): ${reason}`;
    ledger(`## ${note}`);
    verdict(false, note);
    saveRun(state);
    return AGAIN;
  }
  const hand = [kind, reason];
  const same = c.last_hand.length === 2 && c.last_hand[0] === kind && c.last_hand[1] === reason;
  if (f.hold && HOLD_KINDS.has(kind) && !same) {
    carry(state, { last_hand: hand });
    saveRun(state);
    verdict(true, `${reason}\n       The run is HOLDING, not ending. Answer it, then press p -- or delete ${PAUSE} -- to carry on with a fresh session; s ends the run.`);
    CONTROL.armHold(`${kind}: ${reason}`);
    const stop = await holdPause(ledger);
    if (stop) return finish(state, `${reason}, and then: ${stop}`);
    say("the hold is lifted -- another session, on the tree as it stands", C.GREEN);
    ledger(`## run held on ${kind} and carried on -- ${reason}`);
    return AGAIN;
  }
  if (same) return finish(state, `${reason} -- twice, with a hold in between, so the run ends rather than asking the same question again`);
  return finish(state, reason);
}

/**
 * Sleeps until the wall reopens, with a minute's margin for the server's clock, and `r` or `.loop/retry`
 * ends the wait at once. Returns "" when the run may go on, or the reason it ends: a stop, or a wall
 * further out than `maxWait` seconds.
 */
async function waitWall(limit: RateLimit, maxWait: number): Promise<string> {
  let left = limit.left() + 60;
  if (left <= 0) {
    rmSync(join(ROOT, LIMIT), { force: true });
    return "";
  }
  if (left > maxWait) {
    return `${limit.describe()} -- further out than --max-limit-wait (${hms(maxWait)}), so the run stops here rather than sleeping through it. Nothing is lost: every session committed its own slices, and a restart after ${limit.when()} picks up from the handoff`;
  }
  rememberLimit(limit);
  ledger(`       usage wall: ${limit.describe()}; waiting ${hms(left)}`);
  TICKER.set({ phase: "usage wall" });
  // A request older than this wall is not about it.
  rmSync(join(ROOT, RETRY), { force: true });
  step(`press r to retry now -- after switching accounts, say -- or s to stop the run. From another terminal: create ${RETRY} or ${STOP}`, C.CYAN);
  CONTROL.parked = true;
  try {
    while (left > 0) {
      const stop = CONTROL.stopReason();
      if (stop) return `${stop}, while waiting out the usage limit`;
      if (CONTROL.takeRetry()) {
        step("retrying now at your request -- the wall is dropped, and the session it refused runs next", C.GREEN);
        rmSync(join(ROOT, LIMIT), { force: true });
        return "";
      }
      await wait(left, () => CONTROL.pending());
      left = limit.left() + 60;
    }
  } finally {
    CONTROL.parked = false;
  }
  rmSync(join(ROOT, LIMIT), { force: true });
  step(`the usage window has reopened -- ${limit.when()} has passed`, C.GREEN);
  return "";
}

/**
 * The command `STAND_IN_ENV` names in place of `claude`, split on whitespace, or null when it is unset. A
 * rehearsal runs a turn against a script that plays a session this way, and the arguments a real session
 * takes are appended to it all the same.
 */
function standIn(): string[] | null {
  const words = (process.env[STAND_IN_ENV] ?? "").trim().split(/\s+/).filter(Boolean);
  return words.length > 0 ? words : null;
}

/** The count the store's `meta` holds under `key`, or `missing` when it holds none. */
function metaCount(key: string, missing: number): number {
  const store = new SelectStore();
  try {
    const raw = store.meta(key);
    const since = raw === null ? NaN : Number(raw);
    return Number.isInteger(since) && since >= 0 ? since : missing;
  } finally {
    store.close();
  }
}

function setMetaCount(key: string, count: number): void {
  const store = new SelectStore();
  try {
    store.setMeta(key, String(count));
  } finally {
    store.close();
  }
}

/** Turns since the floor gate last opened; a store that holds no count opens it, the safe direction. */
function floorSince(): number {
  return metaCount(FLOOR_META, FLOOR_GATE_EVERY);
}

/**
 * `bun nv select --full` inside a turn: every atom that is not heavy runs and records, and the store's
 * tree moves past the change. `fullRun` takes the sweep lock itself while a plan check needs the origin.
 * Returns a red line naming the selection misses, or a covws build that failed, and "" when there is
 * neither.
 */
async function fullRunTurn(note: (line: string) => void): Promise<string> {
  const store = new SelectStore();
  try {
    const r = await fullRun(store, { say: note });
    for (const line of describeFull(r)) note(line);
    if (r.buildFailed) return "full run: the covws build failed";
    if (r.misses.length === 0) return "";
    const named = r.misses.slice(0, 5).map((m) => m.id).join(", ");
    return `full run: ${r.misses.length} selection miss(es), red now and not selected by the change that broke them: ${named}${r.misses.length > 5 ? ", ..." : ""}`;
  } finally {
    store.close();
  }
}

function setFloorSince(since: number): void {
  setMetaCount(FLOOR_META, since);
}

/**
 * One turn of the run; the module doc says what it does and what it does not do yet. It owns the console
 * for its length: the live block, the keys, and a Ctrl-C, which ends the session where it stands, sweeps
 * what it left into a wip commit and ends the run.
 */
async function turn(f: TurnFlags): Promise<number> {
  const name = process.env[RUN_ENV]!;
  const held = holder();
  if (held !== "" && held !== name) {
    say(`nv loop: run \`${held}\` holds this tree (${MARKER}); a second driver would edit it beside that one`, C.RED);
    return 2;
  }
  const { state, fresh } = loadRun(name);
  CONSOLE.openRun(join(ROOT, LOGDIR, `${state.run_id}-console.log`));
  const touched = new Touched();
  const ctx: TurnContext = { index: 0, live: null, ending: false };
  const interrupt = async () => {
    if (ctx.ending) return;
    ctx.ending = true;
    ctx.live?.tree.kill();
    say("");
    const swept = await markInterrupted(ctx.index, "the run was interrupted with Ctrl-C", touched);
    say(
      swept.committed ? `interrupted -- ${swept.paths} uncommitted path(s) swept into a wip commit` : "interrupted -- the session left nothing uncommitted of its own",
      C.YELLOW,
    );
    ledger(`## run ended ${stamp()} -- interrupted (Ctrl-C)${swept.committed ? `; swept ${swept.paths} path(s) into a wip commit` : ""}`);
    if (holder() === state.run) rmSync(join(ROOT, MARKER), { force: true });
    CONTROL.dropPause();
    TICKER.stop();
    CONTROL.disable();
    process.exit(0);
  };
  const onSignal = () => void interrupt();
  process.on("SIGINT", onSignal);
  CONTROL.onInterrupt = onSignal;
  CONTROL.enable();
  if (f.status) TICKER.start();
  let code = 1;
  try {
    const out = await serve(f, state, fresh, touched, ctx);
    code = typeof out === "number" ? out : await afterTurn(state, f, out);
    return code;
  } finally {
    // Every way out but the next turn ends the run, and a run that ended holds the tree no longer:
    // `finish` drops the marker too, and this catches a turn that returned or threw without it.
    if (code !== AGAIN && holder() === state.run) rmSync(join(ROOT, MARKER), { force: true });
    process.off("SIGINT", onSignal);
    TICKER.stop();
    CONTROL.disable();
  }
}

/** What the Ctrl-C handler needs of the turn in flight. */
interface TurnContext {
  index: number;
  live: LiveSession | null;
  ending: boolean;
}

/**
 * The turn inside the console `turn` set up: one session, its sweep, and whether the run goes on. Returns
 * `AGAIN` when it does, the exit code when the run ends here, or the verdict `afterTurn` settles.
 */
async function serve(f: TurnFlags, state: RunState, fresh: boolean, touched: Touched, ctx: TurnContext): Promise<number | Ended> {
  if (fresh) {
    // At the door, before the marker: a run that fills the disk dies inside a session with the tree half edited.
    const disk = enoughDisk(f.minFreeGb);
    if (disk) {
      say(disk, C.RED);
      return 2;
    }
    writeFileSync(join(ROOT, MARKER), `pid:      ${process.pid}\nrun:      ${state.run}\nstarted:  ${stamp()}\nsessions: ${Number.isFinite(f.maxSessions) ? f.maxSessions : "uncapped"}, model ${f.model}\n`);
    ledger("");
    ledger(`## run started ${stamp()} (max ${Number.isFinite(f.maxSessions) ? `${f.maxSessions} sessions` : "uncapped"}${f.effort ? `, effort ${f.effort}` : ""}, logs ${state.run_id}-*)`);
    say(`console log: ${LOGDIR}/${state.run_id}-console.log`, C.GRAY, true);
    // The key row says this all the time, so it is a line only when that row is not painted.
    if (!(CONTROL.tty && TICKER.enabled)) {
      say(
        `controls: ${
          CONTROL.tty
            ? "press r to end a usage wait early, s to stop after the current session, p to hold after it without ending the run, h to halt the running session at once and again to carry on, i to type it a prompt"
            : `create ${RETRY} to end a usage wait early, ${STOP} to stop after the current session, ${PAUSE} to hold after it until the file goes, ${HALT} to freeze the running session until the file goes, ${SAY} with a prompt in it to send it one`
        }`,
        C.GRAY,
        true,
      );
    }
  }
  // The code this turn judges with, taken before a session can change it: `driverChanged`.
  const imported = driverFiles();
  // A session the last turn served and could not judge, because it changed that code. This turn serves
  // no session: it writes the sweep into that session's log and judges it below.
  const pending = pendingJudge(state);
  state.judge = {};
  if (state.served >= f.maxSessions && pending === null) return end("budget", `hit --max-sessions (${f.maxSessions})`);
  const found = selected({});
  if (found === null) return end("chain-error", "the live goal's plan did not read; `bun nv check` and `bun nv chain --check` say why");
  const { live, goal, labelOf, total } = found;
  if (fresh) step(live.num === null ? `side run: goal \`${live.slug}\`, ${placeOf(live, total)}` : `chain: resuming at goal \`${live.slug}\`, ${placeOf(live, total)}`, C.CYAN);
  // On every turn: a Docker Desktop a session started is in that turn's job and ends when the turn exits,
  // so the daemon is checked, and started again if it is gone, before each session and each sweep.
  const down = (await preflight(goal.env.docker, (l) => say(`   ${l}`, C.GRAY))) || (await bringUp(goal.env.docker, (l) => say(`   ${l}`, C.GRAY)));
  if (down) return finish(state, `chain: ${down}`);
  const checks = goal.checks as Check[];
  const plan = { slug: live.slug, stages: goal.stages, checks };
  const results: Results = await lastGreen(checks);

  const session = new Session(plan, results);
  // The status line is the goal's row. Between sessions its last field is the driver's phase. The
  // progress directory is read at most twice a second, since the row is asked for more often.
  let working: string[] = [];
  let workingAt = 0;
  TICKER.row = (width) => {
    if (ctx.live === null) session.phase(TICKER.phase());
    if (performance.now() - workingAt > 500) {
      working = readProgress(PROGRESS_DIR);
      workingAt = performance.now();
    }
    return statusRow(plan, results, session, width, working, live.num === null ? "side goal" : `${live.num} of ${total}`);
  };
  CONTROL.onGoal = () => {
    for (const row of goalTable({ plan, results, session, position: live.num, total, commits: SLICES.subjects(), width: TICKER.width() - 1, utf8: TICKER.utf8 })) say(row);
  };
  const scope = `${state.served + 1}${Number.isFinite(f.maxSessions) ? `/${f.maxSessions}` : ""}`;
  const exe = standIn() ?? [Bun.which("claude") ?? "claude"];
  const renderer = new Renderer(f, f.effort ?? "");
  const sweptBy = (swept: Swept) =>
    swept.paths === 0
      ? "; the tree is clean"
      : swept.committed
        ? `; ${swept.paths} path(s) swept into a wip commit`
        : `; ${swept.paths} path(s) left uncommitted, the sweep's commit failed`;

  // A session the account refused or the CLI dropped is swept and run again, in this turn, until one is
  // served or the run ends. `index` goes up on every launch, so no log is written twice.
  let wall = standingLimit();
  if (wall !== null) step(`${LIMIT} says ${wall.describe()}`, C.YELLOW);
  let walls = 0;
  let fails = 0;
  // Sessions the API refused as overloaded in a row, and when that streak began.
  let overloads = 0;
  let overloadedSince = 0;
  // Resumes in a row of one dropped session, and the session id the next launch rejoins.
  let resumes = 0;
  let resumeFrom = "";
  let repaired = false;
  let index = 0;
  let number = "";
  let base = "";
  let operator = "";
  let line = "";
  let commits = 0;
  if (pending !== null) {
    index = ctx.index = pending.index;
    number = String(index).padStart(4, "0");
    ({ line, commits, base } = pending);
    SLICES.start(base);
    session.begin(index);
    CONSOLE.openSession(join(ROOT, LOGDIR, `${state.run_id}-${number}.log`));
    step(`judging session ${index} with the driver code it committed -- no session this turn`, C.CYAN);
  }
  for (; pending === null; ) {
    const asked = CONTROL.stopReason();
    if (asked) return end("asked", asked);
    if (wall !== null) {
      const stop = await waitWall(wall, f.maxLimitWait);
      if (stop) return end("wall-timeout", stop);
      wall = null;
    }
    // After the wall, since a hold is a promise about the tree: one queued during a wait is honoured when the window reopens.
    const holdStop = await holdPause(ledger);
    if (holdStop) return end("asked", holdStop);
    state.index++;
    index = ctx.index = state.index;
    number = String(index).padStart(4, "0");
    saveRun(state);
    const log = join(ROOT, LOGDIR, `${state.run_id}-${number}.log`);
    // Consumed here whatever happens below, so a launch only ever rejoins the transcript the drop branch chose.
    const rejoin = resumeFrom;
    resumeFrom = "";
    // A rejoined session is the same session mid-slice: its status, its base and its pack are still its own.
    if (!rejoin) {
      rmSync(join(ROOT, RUNDIR, "status.txt"), { force: true });
      base = await head();
      SLICES.start(base);
    }
    say(`== session ${scope}  ${clock()}`, C.CYAN);
    let pack = "";
    let opening: string;
    if (rejoin) {
      step(`resuming session ${rejoin} -- no pack, its transcript already carries one`, C.CYAN);
      opening = RESUME_PROMPT;
    } else {
      step("building the orientation pack (bun nv orient)");
      TICKER.set({ phase: "orienting" });
      const orientedAt = performance.now();
      const oriented = await runProc([process.execPath, join(ROOT, "tools/nv/main.ts"), "orient"], { timeoutMs: 120_000 });
      pack = oriented.code === 0 ? oriented.stdout : "";
      const spent = mmss((performance.now() - orientedAt) / 1000);
      if (pack) step(`orientation pack: ${Buffer.byteLength(pack).toLocaleString("en-US")} bytes in ${spent}`);
      else step(`orientation pack: nv orient failed after ${spent} -- the session will run it itself`, C.YELLOW);
      // A run repairing a verdict hands this session the repair prompt and the verdict instead of a goal item.
      const repair = carried(state).repair;
      opening = readFileSync(join(ROOT, repair ? REPAIR_PROMPT : PROMPT), "utf8");
      if (repair) {
        opening = `${opening.trimEnd()}\n\n\`\`\`\n${repair}\n\`\`\`\n`;
        step("a repair session: it gets the verdict the run stopped on, not a goal item", C.CYAN);
      }
    }

    // The rules earlier sessions earned, committed before this one starts, which is when the harness reads them.
    const allowed = standIn() ? { added: [] } : await settleAllow(loadSeen());
    if ("skipped" in allowed) step(`allowlist: owed rules wait -- ${allowed.skipped}`, C.YELLOW);
    else if (allowed.added.length > 0) {
      step(`allowlist: added ${allowed.added.join(", ")}`, C.CYAN);
      ledger(`- ${number} allowlist: the driver added ${allowed.added.map((r) => `\`${r}\``).join(", ")}`);
    }
    const allowWatch = new AllowWatch();

    const effort = f.effort ? `, --effort ${f.effort}` : "";
    step(`launching ${exe.join(" ")} (--model ${f.model}${effort}, --permission-mode ${f.permissionMode}${rejoin ? `, --resume ${rejoin}` : ""})`);
    TICKER.set({ phase: "launching" });
    session.begin(index);
    renderer.begin();
    touched.start(rejoin !== "");
    VERIFY.arm();
    const startedAt = performance.now();
    let latest: RateLimit | null = null;
    const launched = await launch(
      exe,
      { ...f, ...(rejoin ? { resume: rejoin } : {}) },
      openingLine(opening, pack),
      log,
      { bytes: Buffer.byteLength(pack), goal: live.slug },
      (e) => {
        latest = readLimit(e) ?? latest;
        touched.note(e);
        allowWatch.note(e);
        session.feed(e);
        renderer.event(e);
      },
      { [WRITES_ENV]: touched.ledger },
      (child) => {
        // After `launch` has written the log's first line, which `loop-stats` reads as the pack's size.
        CONSOLE.openSession(log);
        ctx.live = new LiveSession(new ProcTree(child), child.write, child.open);
        CONTROL.attach(ctx.live);
      },
    );
    VERIFY.disarm();
    operator = ctx.live?.record() ?? "";
    ctx.live?.finish();
    ctx.live = null;
    CONTROL.detach();
    TICKER.set({ phase: "closing the session" });
    consoleMode();
    const tokens = renderer.tokens();
    step(`session ${index} ended after ${mmss((performance.now() - startedAt) / 1000)}, claude exit ${launched.code}${tokens ? `, ${tokens}` : ""}`, C.CYAN);
    const where = log.slice(ROOT.length + 1).replace(/\\/g, "/");
    if (!standIn()) {
      const seen = loadSeen();
      const outcome = allowWatch.outcome();
      const refused = new Set(recordAllow(seen, launched.sessionId || where, outcome));
      saveSeen(seen);
      for (const { key, command } of outcome.blocked) {
        if (!refused.has(key)) continue;
        const [tool, prefix] = key.split("\t");
        ledger(`- ${number} allowlist: refused \`${command.split("\n")[0]!.slice(0, 160)}\` -- \`${tool}(${prefix}:*)\` is never added by the driver; add it to .claude/settings.json by hand if it is wanted -- see ${where}`);
      }
    }

    // The wall is judged before the exit code, because it explains it: a refused session exits non-zero
    // exactly like a crashed one, and neither a retry nor the tree can help with it.
    const limit = wallAfter(latest, launched.code, launched.result);
    if (limit !== null) {
      walls++;
      const swept = await markInterrupted(index, limit.describe(), touched);
      ledger(`- ${number} refused by the usage wall -- ${limit.describe()}${sweptBy(swept)} -- see ${where}`);
      if (walls >= MAX_WALLS) return end("wall", `${walls} sessions in a row were refused by the usage limit`);
      wall = limit;
      continue;
    }
    // An overload explains its exit as the wall does: the server is busy, there is no deadline to sleep to
    // and nothing in the tree to fix, so the session runs again for as long as it takes, costing no retry.
    const status = apiErrorStatus(launched.result);
    if (launched.code !== 0 && OVERLOAD_STATUS.has(status)) {
      overloads++;
      if (overloads === 1) overloadedSince = startedAt;
      const back = nthWait(OVERLOAD_BACKOFF, overloads);
      const swept = await markInterrupted(index, `the API answered ${status} Overloaded`, touched);
      ledger(`- ${number} refused by the API -- ${status} Overloaded${sweptBy(swept)}; overloaded ${hms((performance.now() - overloadedSince) / 1000)} so far, retry ${overloads + 1} in ${hms(back)} -- see ${where}`);
      step(`the API answered ${status} Overloaded -- the server is busy, not the account, so this session runs again rather than counting as a crash. Retry ${overloads + 1} in ${hms(back)}; r retries now`, C.YELLOW);
      TICKER.set({ phase: "waiting out an API overload" });
      rmSync(join(ROOT, RETRY), { force: true });
      CONTROL.parked = true;
      try {
        await wait(back, () => CONTROL.pending());
        if (CONTROL.takeRetry()) step("retrying now at your request -- the backoff is dropped", C.GREEN);
      } finally {
        CONTROL.parked = false;
      }
      continue;
    }
    // A dropped stream left a healthy session whose transcript is whole, so it is rejoined, and the tree is
    // left as it was: the session that comes back is the one that can tell its unfinished slice apart.
    if (launched.code !== 0 && streamDropped(launched.result) && launched.sessionId && resumes < MAX_RESUMES) {
      resumes++;
      const back = nthWait(RESUME_BACKOFF, resumes);
      resumeFrom = launched.sessionId;
      ledger(`- ${number} the connection dropped mid-response -- rejoining its transcript, attempt ${resumes}/${MAX_RESUMES} in ${mmss(back)}; the tree is left as the session had it -- see ${where}`);
      step(`the connection dropped mid-response -- the session is intact on disk, so it is resumed. Attempt ${resumes}/${MAX_RESUMES} in ${mmss(back)}`, C.YELLOW);
      TICKER.set({ phase: "resuming a dropped session" });
      await wait(back);
      continue;
    }
    if (launched.code !== 0) {
      fails++;
      resumes = 0;
      const swept = await markInterrupted(index, `the CLI exited ${launched.code}`, touched);
      ledger(`- ${number} CLI exit ${launched.code} (attempt ${fails}/${f.maxRetries})${sweptBy(swept)} -- see ${where}`);
      if (fails >= f.maxRetries) return end("cli-failed", `claude CLI failed ${fails} times in a row`);
      const back = backoff(fails);
      step(`backing off ${mmss(back)} before retry ${fails + 1}`, C.YELLOW);
      TICKER.set({ phase: `backing off before retry ${fails + 1}` });
      await wait(back);
      continue;
    }
    break;
  }
  if (pending === null) {
    state.served++;
    // Spent on the session that was served, so a repair the usage wall refused is still owed.
    repaired = carried(state).repair !== "";
    carry(state, { repair: "" });
    saveRun(state);

    const statusPath = join(ROOT, RUNDIR, "status.txt");
    line = existsSync(statusPath) ? readFileSync(statusPath, "utf8").trim() : "";
    // A session that exits zero without wrapping, cut off by the harness or out of turns, leaves its
    // unfinished slice as surely as a crashed one, and one that wrapped leaves nothing and closes an earlier
    // interruption.
    const swept = await markInterrupted(index, "it exited without wrapping", touched);
    if (swept.committed) step(`swept ${swept.paths} uncommitted path(s) into a wip commit -- the session ended without wrapping`, C.YELLOW);
    const counted = await runProc(["git", "rev-list", "--count", `${base}..HEAD`]);
    commits = Number(counted.stdout.trim()) || 0;
    const left = swept.left > 0 ? ` | ${swept.left} path(s) the session never wrote left in the tree` : "";
    const wip = swept.committed ? ` | ${swept.paths} path(s) swept into a wip commit` : swept.paths > 0 ? ` | ${swept.paths} path(s) left uncommitted, the sweep's commit failed` : "";
    // Said because it changes what the line means: a session a person halted or spoke to is not an unattended one.
    const attended = (operator ? ` | ${operator}` : "") + (repaired ? " | repair session" : "");
    ledger(`- ${number} ${commits} commit(s)${wip}${left}${attended} | ${line || "(no status written)"}`);

    // Straight to the next turn: no budget or stall check stands between a session and its verdict.
    const stale = driverChanged(imported);
    if (stale.length > 0) {
      state.judge = { index, line, commits, base };
      const reason = `session ${index} changed the driver's own code (${stale.join(", ")}), so a fresh turn judges it`;
      ledger(`       ${reason}`);
      CONSOLE.closeSession();
      saveRun(state);
      say(reason, C.CYAN);
      return AGAIN;
    }
  }

  // The sweep, over the goal's list as the session left it.
  const again = selected({});
  if (again === null) return end("chain-error", "the goal's plan did not read after the session; `bun nv check` and `bun nv chain --check` say why");
  const all = again.goal.checks as Check[];
  const since = floorSince() + 1;
  let open = since >= FLOOR_GATE_EVERY;
  // Every check but a heavy one: what starts is what this session's change reached, carried or the
  // goal's own, and what was owed before it. A setup command is never held, since the fixtures read
  // what it writes.
  const heldBack = open ? [] : all.filter(heldByGate);
  const scoped = all.filter((c) => !heldBack.includes(c));
  // On the floor gate's cadence the full run comes first: every atom that is not heavy runs and records,
  // so the sweep after it reaches the heavy checks, the Linux legs and whatever is red. A selection miss
  // it finds is a red of this sweep.
  let missed = "";
  if (open) {
    step("full run: every atom that is not heavy runs and records, and a red one the selection did not pick is a selection miss", C.CYAN);
    TICKER.set({ phase: "full run" });
    missed = await fullRunTurn((l) => say(`   ${l}`, C.GRAY));
    if (missed) ledger(`       ${missed}`);
  }
  step(`acceptance check: every check a change reached${open ? ", the heavy ones and the Linux legs with them" : ` (heavy checks and the Linux legs held, 1 session in ${FLOOR_GATE_EVERY})`}`, C.CYAN);
  const checkedAt = performance.now();
  const progress = sweepProgress(again.labelOf, checkedAt);
  const onDone = (c: Check, ok: boolean) => results.set(c.id, ok);
  TICKER.set({ phase: "acceptance sweep", total: scoped.length, done: 0 });
  // Every red is collected: a carried check that went red must not keep the goal's own checks behind it
  // from being judged.
  let { result, secs } = await sweepOver(again.goal, scoped, again.labelOf, { full: false, collect: true, legs: true, gateOpen: open, onDone, progress });
  let cost = `${secs}s over ${result.ran} check(s), ${result.answered} not reached`;
  // Only a sweep that would reach the goal pays for the two goal-end gates. They ask about the goal's own
  // work, which the scoped sweep has just passed, so they run on the gate-open sweep red or green, and one
  // session sees every finding.
  let reaching = result.fail === "";
  if (result.fail === "" && heldBack.length > 0) {
    // On the safety net's cadence of goal ends, every heavy check and leg runs, whatever this goal changed.
    const goalEnds = metaCount(HEAVY_FULL_META, HEAVY_FULL_EVERY) + 1;
    const heavyAll = goalEnds >= HEAVY_FULL_EVERY;
    const which = heavyAll ? `every heavy check and leg, 1 goal end in ${HEAVY_FULL_EVERY}` : "what this goal changed";
    step(`scoped sweep green with ${heldBack.length} check(s) held -- opening the floor gate over ${which} before the goal is reached`, C.CYAN);
    ledger(`       goal cost: ${cost}, ${heldBack.length} held (scoped; opening the floor gate${heavyAll ? " over every heavy check" : ""})`);
    open = true;
    reaching = true;
    TICKER.set({ phase: "acceptance sweep, floor gate open", total: all.length, done: 0 });
    ({ result, secs } = await sweepOver(again.goal, all, again.labelOf, { full: false, collect: true, legs: true, gateOpen: true, heavyAll, onDone, progress }));
    cost = `${secs}s over ${result.ran} check(s), ${result.answered} not reached`;
    setMetaCount(HEAVY_FULL_META, heavyAll ? 0 : goalEnds);
  }
  setFloorSince(open ? 0 : since);
  if (missed) result = { ...result, fail: result.fail ? allReds([result.fail, missed]) : missed };
  step(`acceptance check done in ${mmss((performance.now() - checkedAt) / 1000)}`, C.CYAN);
  ledger(`       goal cost: ${cost}${open ? "" : `, ${heldBack.length} held (floor gate shut)`}`);
  // Every session: the sweep has just left the build warm, nothing is building, and a day of builds is what fills `target/`.
  TICKER.set({ phase: "cleaning disk" });
  ledger(`       ${await sweepDisk(f.keepRuns, (l) => say(`   ${l}`, C.GRAY))}`);
  const gateSay = (l: string) => step(l, C.CYAN);
  const docsRed = reaching ? await docGate(number, gateSay) : "";
  if (docsRed) ledger(`       doc gate: ${docsRed}`);
  const ownerRed = reaching ? await ownerGate(live.slug, number, gateSay) : "";
  if (ownerRed) ledger(`       owner gate: ${ownerRed}`);
  TICKER.set({ phase: "between sessions" });
  // The verdict on this session is the last thing that belongs in its log.
  CONSOLE.closeSession();
  if (result.fail === "" && !docsRed && !ownerRed) {
    ledger(`## goal reached: ${live.slug} -- every check in its acceptance list passes`);
    say(`GOAL REACHED: ${live.slug}`, C.GREEN);
    // A side goal is never switched from: its run ends, and a person lands the branch.
    if (live.num === null) return end("side-green", landing(live.slug, goal.env.wsl?.targetDir ?? null));
    const switched = await advance(state, live.slug);
    if (switched !== null) return switched;
    verdict(false, `goal reached -- the run carries on with \`${liveGoal()?.slug ?? "?"}\``);
    carry(state, { last_hand: [] });
    saveRun(state);
    return AGAIN;
  }
  if (result.fail !== "") ledger(`       goal check: ${result.fail}`);

  // A DONE held only by a goal-end gate is not a wrong claim, just an unfinished one: the next pack carries
  // the finding. A DONE the sweep refuses gets one retry session with the failing check in its pack, and a
  // hand is asked when the retry fails on the check it was handed or the goal has spent its retries. A retry
  // that closed its check and fell to a different one made progress, so it is a new question.
  const c = carried(state);
  const handed = c.retry_check;
  carry(state, { retry_check: "" });
  if (line.startsWith("DONE") && result.fail !== "") {
    const failing = checkOf(result.fail);
    if (failing === handed) return end("done-claim", `session reported DONE but the acceptance test does not pass yet, and the retry failed the check it was handed, \`${failing}\`: ${line}`);
    if (c.done_retries >= DONE_RETRIES) return end("done-claim", `session reported DONE but the acceptance test does not pass yet, and the goal has spent its ${DONE_RETRIES} retries: ${line}`);
    carry(state, { done_retries: c.done_retries + 1, retry_check: failing });
    const progressed = handed ? `; the retry closed \`${handed}\` and fell to a different check, so it is a new question` : "";
    const retried = `done-claim retried: the DONE above failed its sweep, and one session gets the failing check before a hand is asked (${c.done_retries + 1} of ${DONE_RETRIES} on this goal)${progressed}`;
    step(retried, C.YELLOW);
    ledger(`       ${retried}`);
  }
  if (line.startsWith("BLOCKED")) return end("blocked", `blocked on a user decision: ${line}`);
  state.stalls = commits === 0 ? state.stalls + 1 : 0;
  if (state.stalls >= f.maxStalls) return end("stalled", `${state.stalls} sessions in a row produced no commit`);
  if (state.served >= f.maxSessions) return end("budget", `hit --max-sessions (${f.maxSessions})`);
  carry(state, { last_hand: [] });
  verdict(false, line || "the session wrote no status line, and the loop carries on");
  // The key's flag lives in this process, so a stop pressed in the turn's last seconds is settled here.
  await CONTROL.settleStop();
  const asked = CONTROL.stopReason();
  if (asked) return end("asked", asked);
  saveRun(state);
  return AGAIN;
}

/**
 * The goal switch, once goal `from` is reached: `leaveGoal` deletes its prose, record and handoff record, takes
 * its slug off the chain and makes the next goal live, and all of it is committed with the goal plan it changes
 * and nothing else. Then that goal's services come up, and the counts a goal owns start over. Nothing is
 * archived (`rule:tooling/the-chain-names-its-live-goal`). Returns null when the run goes on, or the verdict:
 * the chain is complete, or the next goal cannot be run.
 */
async function advance(state: RunState, from: string): Promise<Ended | null> {
  const goals = chainGoals();
  const at = goals.findIndex((g) => g.slug === from);
  const next = goals[at + 1];
  if (next === undefined) return end("chain-complete", `CHAIN COMPLETE: \`${from}\` was the last goal in data/chain.json, and every one of them is green`);
  say("");
  step(`chain: switching to goal \`${next.slug}\`, ${next.num} of ${goals.length}`, C.CYAN);
  const plan = goalPlan(next.slug);
  if (plan === null) return end("chain-error", `chain: goal \`${next.slug}\` has no record at data/goals/${next.slug}.json`);
  if (plan.checks.length === 0) return end("chain-error", `chain: goal \`${next.slug}\` has no checks, so nothing can reach it; \`bun nv chain --check\` says what it owes`);
  const issues = goalType.schema.validate(plan);
  if (issues.length > 0) return end("chain-error", `chain: goal \`${next.slug}\`'s plan is not runnable -- ${JSON.stringify(issues[0])}`);
  const down = await preflight(plan.env.docker, (l) => say(`   ${l}`, C.GRAY));
  if (down) return end("chain-error", `chain: ${down}`);

  const paths = leaveGoal(from, next.slug);
  // The goal plan lists the chain and marks which goal is live, so it moves in the same commit.
  const rendered = writeGoalPlan();
  const msg = join(ROOT, ".agent-tmp", "chain-switch.txt");
  mkdirSync(dirname(msg), { recursive: true });
  // The subject names the goals and never their positions, which a later insert moves.
  writeFileSync(msg, `docs(loop): the chain advances from \`${from}\` to \`${next.slug}\`\n\nGoal \`${from}\` is deleted: its prose, its record and its handoff record.\n`);
  const committed = await runProc(["git", "commit", "-q", "-F", msg, "--", ...paths.map((p) => join(ROOT, p)), ...(rendered ? [join(ROOT, rendered)] : [])]);
  rmSync(msg, { force: true });
  if (committed.code !== 0) return end("chain-error", `chain: \`${next.slug}\` is live in data/chain.json, and its commit failed -- ${(committed.stderr || committed.stdout).trim().split("\n")[0]}`);

  const up = await bringUp(plan.env.docker, (l) => step(l, C.CYAN));
  if (up) return end("chain-error", `chain: ${up}`);
  // `from` left the chain, so every place behind it moved up by one.
  const place = `${next.num - 1} of ${goals.length - 1}`;
  ledger(`## run continues on goal \`${next.slug}\` (${place})`);
  say(`chain: goal \`${next.slug}\` is live, ${place}`, C.GREEN);
  // A new goal is a new worklist: a stall streak, its DONE retries and its repairs start over.
  state.stalls = 0;
  carry(state, { done_retries: 0, retry_check: "", repairs: 0 });
  return null;
}

/** What a sweep inside a turn says: one grey line per check and per process it starts, stamped with the sweep's clock. */
function sweepProgress(labelOf: (n: number) => string, begun: number): Progress {
  const at = () => `+${mmss((performance.now() - begun) / 1000).padStart(6)}`;
  return {
    run: (what) => say(`   .. ${at()}  ${what}`, C.GRAY),
    trace: (c) => {
      TICKER.advance();
      say(`   .. ${at()}  ${nameOf(c)} [${labelOf(c.stage)}]`, C.GRAY);
    },
    note: (text) => say(`   .. ${" ".repeat(7)}  ${text}`, C.GRAY),
    count: (steps) => TICKER.grow(steps),
    plan: (steps) => TICKER.reach(steps, "linux legs"),
    step: () => TICKER.advance(),
  };
}

/** Where every `nv` command a session or a check starts writes what it is doing; the status row reads it. */
const PROGRESS_DIR = join(ROOT, RUNDIR, "progress");

export async function run(args: string[]): Promise<number> {
  process.env[PROGRESS_ENV] = PROGRESS_DIR;
  // `--side <slug>` goes first and says which goal every mode below works on, through the variable the
  // turns and their sessions inherit.
  let side = "";
  if (args[0] === "--side") {
    side = args[1] ?? "";
    process.env[SIDE_ENV] = side;
    if (sideGoal() === null) {
      console.error(side ? `nv loop --side: there is no side goal \`${side}\` at docs/agent/goals/side/${side}.md` : `usage: ${USAGE}`);
      return 2;
    }
    args = args.slice(2);
  }
  if (args.length === 1 && args[0] === "--goal") return await goalView();
  if (args.length === 1 && args[0] === "--owed") return owed();
  if (args.length === 1 && args[0] === "--settle") return settle();
  const mode = args[0];
  if (mode === undefined || !["--list", "--run", "--goal-only"].includes(mode)) {
    const flags = parseTurn(args);
    if (flags === null) {
      console.error(`usage: ${USAGE}`);
      return 2;
    }
    // Started by hand, this process is the run and every turn is a child of it; `driver/respawn.ts`. A
    // side run started outside its worktree is handed over to the driver inside it; `driver/side.ts`.
    if (process.env[RUN_ENV]) return turn(flags);
    if (side && !(await insideWorktree(side))) return launchSide(side, args, flags.minFreeGb, (l) => console.log(l));
    return respawn(args);
  }
  const filters = mode === "--list" || mode === "--run" || mode === "--goal-only" ? parse(args) : null;
  if (filters === null) {
    console.error(`usage: ${USAGE}`);
    return 2;
  }
  if (mode === "--goal-only") return goalOnly(filters);
  return mode === "--run" ? runChecks(filters) : list(filters);
}
