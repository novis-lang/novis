// The turn driver: `Driver.turn` is one turn of a main loop, from the gate to the decision, and
// `RunState` is all a run carries from one turn to the next. It ports `crates/looper-engine/src/driver.rs`,
// with a side loop's catch-up in `catch-up.ts`. The landing that follows a catch-up is the worker's.
//
// # One turn
//
// 1. **Gate.** A usage wall standing on the loop's account (`wall.ts`), which it first reads from
//    looper's service when one answers, ends the turn with a `wait` before anything starts. The wait
//    lasts until the wall clears, or `max_limit_wait_s` when that is sooner, and the next turn reads
//    the wall again. A `retry-now` ends the wait sooner. An empty
//    chain ends the run, and so does a run that has started the `max_sessions` a `set` gave it. Stop,
//    pause and the `before_run` and `before_session` hooks are the caller's gate, and so are the
//    `after_session` hooks after the turn.
// 2. **Pick.** The live goal is the one `liveGoal` names and the live stage its first open stage. A side
//    loop (`Side`) holds instead when the live goal is not its own, such as when its checkout has left
//    `side/<slug>`. A side loop in the project root (`ownGoal`) has its goal made live by
//    `LOOPER_SIDE_GOAL` in its worker's environment, holds the same way when it is not, and ends the run
//    once its goal file is gone. The run's `Mode` says whether the session is ordinary work, the retry of a refused `DONE`, or a repair.
//    A dropped stream left to rejoin outranks all three. After a `skip-stage`, the next fresh session
//    takes the open stage after the live one, or the live one when there is none after it. No goal file
//    changes.
// 3. **Open.** The opening is the prompt for the mode (`Driver.prompt`) with the orientation pack behind
//    it, or, to rejoin a dropped stream, `RESUME_PROMPT` alone. A skipped stage is named in the prompt,
//    and the pack shows the stage taken as the live one. The pack's project sections run with the
//    loop's variables, and `LOOPER_STAGE` names that stage. After them comes the section on the loop's
//    decisions for the user (`packSection` in `decisions.ts`), when any stands.
// 4. **Serve.** The session gets the next number of the loop and its folder `sessions/<n>/`, whose
//    `session.json` records it from its start. It is served there (`serve` in `session.ts`) with
//    `LOOPER_LOOP`, `LOOPER_GOAL`, `LOOPER_ROOT` for the root that holds the loop's folder, and
//    `LOOPER_SIDE_GOAL` for a side loop in the root. Its
//    edit claims are released once it ends. Then the driver reads `looper.toml` again (`Driver.reload`), so every step after the session runs the config the session
//    left. One that does not load is a warning, and the turn keeps its config.
// 5. **Classify** the exit through the adapter. A session the kit killed itself is not classified: it is
//    swept and judged like one that ended.
//    - a usage wall is recorded on the account and reported to the service, the session is swept,
//      and the run waits it out;
//      `Policy.maxWalls` walls in a row end the run;
//    - an overload is swept and backed off by `Policy.overloadBackoff`;
//    - a dropped stream is not swept: the next turn rejoins the same agent session with `--resume`,
//      after `Policy.resumeBackoff`, up to `Policy.maxResumes` times, and that session's sweep takes the
//      paths of both;
//    - any other failure that left no status line is swept and backed off, doubling from
//      `Policy.failureBackoff`; `max_retries` in a row become a `cli-failed` repair.
// 6. **Sweep** the session's own uncommitted paths (`sweep` in `turn.ts`). When the session wrote no
//    status line and the sweep made a commit, `.loop/interrupted.json` names it. A session that wrote a
//    status line deletes the file. A session that ended inside a merge is not swept. The catch-up that
//    follows a landing session finishes that merge or holds on its conflicts, and every other turn holds
//    with `MERGE_HOLD`.
// 7. **Judge.** Read the status line from `Driver.statusFile` and count the commits since the session's
//    base. A chain or a goal file the session left unloadable is a `chain-error` repair. Only a session
//    that claims `DONE` while no decision for the user stands (`standing` in `decisions.ts`) runs the
//    `before_sweep` hooks, and holds when one that pauses the loop fails.
//    Then it runs acceptance, adds its results to the run's check history, and asks `notReached`
//    (decision 0014). Every other session is judged by its status line and its commits alone; it checks
//    its own work with verify, and step 9 may sweep it after the verdict.
// 8. **Decide.**
//    - reached: retire the goal and reset the run's per-goal counts (`advanced`), and the next turn takes
//      the next goal on the chain. Once the retirement commit has landed, the `after_retire` hooks run
//      with `LOOPER_GOAL` naming the retired goal and no `LOOPER_STAGE`. One that pauses the loop holds
//      it before the next goal, and the goal stays retired;
//    - reached on a side loop: retire it the same way in its own branch, run the `after_retire` hooks,
//      and catch up (`Driver.catchUp`). A hook that pauses the loop holds it first, and a resume goes on
//      to the catch-up. A side loop never takes another goal;
//    - reached on a side loop in the project root: retire it in the root as a main loop does, and run
//      the `after_retire` hooks. The turn is `advanced`, and the worker ends the run on it, as the next
//      turn would, since the goal file is gone. It never catches up or lands;
//    - reached, but the retirement failed (such as a `commit-msg` hook that refused its commit): hold
//      with git's reason, with the goal put back and still live;
//    - `BLOCKED`, from any kind of session (`Driver.blocked`): each open parked decision the session
//      asked becomes a blocker (`block` in `decisions.ts`), and the loop holds on their ids with the
//      session's reason. A session that asked none holds on the decisions already open. A line with no
//      open decision is retried once, told to file one, and the second such line in a row holds on the
//      reason alone;
//    - `DONE` while a decision is open: each open one becomes a blocker, and a `decisions` hold;
//    - `DONE` while only answered decisions stand: a retry handed why, and a `decisions` hold on the
//      second such claim in a row;
//    - `DONE` refused: a retry handed the reasons, up to `done_retries`, then a `done-claim` repair;
//    - `max_stalls` sessions in a row without a commit: a `stalled` repair;
//    - otherwise continue.
//
//    A repair is granted while the goal has repairs left (`repairs_per_goal`) and the session that asks
//    for it was not itself a repair; otherwise the run holds. During a catch-up the repair is a landing
//    session, whose opening is the landing prompt and the verdict with no pack, because the goal is
//    retired. Once it ends, the catch-up runs again. The same verdict right after a hold ends the run.
// 9. **Every-session sweep.** With `[loop] sweep = "every"`, a session that did not claim `DONE` is
//    followed by the `before_sweep` hooks and acceptance, each check run only on its `every`. Its red
//    checks go to `.loop/last-sweep.md`. The sweep never retires a goal, and only a hook changes the
//    verdict. A `DONE` claim runs every check whatever its `every`.
//
// Between turns the caller may read `looper.toml` again with `Driver.reload`, lay a `set` control's
// `Settings` over the sessions ahead with `Driver.set`, run acceptance outside a session with
// `Driver.check`, and run a hook event with `Driver.hooks`. Every hook and check the driver runs gets the
// loop's variables (`Driver.vars`) over the project's `[checks] env`. The session ones name the last
// session started, from its start until the caller clears `RunState.session` after its `after_session`
// hooks.
//
// # What lands where
//
// The driver writes no file git tracks, except the generated files a catch-up resolves. The status line is read from `Driver.statusFile` and deleted,
// check output goes to `Driver.checkLogs`, hook output to `Driver.hookLogs`, and each session's
// `stream.ndjson`, events, `LOOPER_WRITTEN` list and `session.json` go to its folder under the loop's
// folder `Driver.dir`. The verdict a retry or a repair is handed, and the operator's notes, travel in the
// opening prompt. In the project's tree it writes only the run directory `.loop/`, which the shared
// `.gitignore` block ignores: `interrupted.json` and `last-sweep.md`, in `.loop/side-<slug>/` for a side
// loop in the root (`runDir`).
//
// # The bounds it keeps
//
// `RunState` holds counts and single values, the last verdict, at most `NOTES_MAX` notes and as many
// warnings, at most `WRITTEN_MAX` paths of a dropped session, and the last result of at most
// `CHECKS_MAX` checks, so a run of any length carries the same state. `session.json` holds one record.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { dir as claimDir, release } from "../claims/claims.ts";
import { goalVars, extra as packExtra } from "../cli/orient.ts";
import { type Config, defaultConfig, loadConfig, type Prompts as Named } from "../goal/config.ts";
import { type Check, type Goal, goalPath, liveGoal, loadChain, loadGoal, Refused, SIDE_GOAL_VAR } from "../goal/model.ts";
import { pack } from "../goal/orient.ts";
import { writeState } from "../store/state.ts";
import { acceptance, named, type Progress, type Project, runReported, unreported, type Units, type Verdict } from "../verify/accept.ts";
import { storeDir } from "../verify/select.ts";
import type { AgentAdapter, ExitClass } from "./adapter.ts";
import { catchUp, type Side } from "./catch-up.ts";
import { block, packSection, readDecisions, standing } from "./decisions.ts";
import { type HookEvent, runHooks } from "./hooks.ts";
import { type Controls, type Limits, type Live, limitsFrom, NOTES_MAX, type Served, type Stats, serve, sessionEnd, WRITTEN_MAX } from "./session.ts";
import { git, interrupted, lastSweep, notReached, retire, sweep, WRITTEN_LIST, wrapped } from "./turn.ts";
import { clearsAt, Walls } from "./wall.ts";

/** The opening of a session that rejoins a dropped stream. */
export const RESUME_PROMPT =
  "looper here: the connection to the API dropped mid-response and this session was rejoined with `--resume`, so the conversation above is yours. Nothing moved under you: the tree is as you left it and looper committed nothing. Your last tool call may or may not have landed; read back whatever it touched. Continue, and finish the way the prompt at the top told you to. Do not re-orient.";

/** The opening prompt of an ordinary session. `{status}` stands for the status file's path. */
export const SESSION_PROMPT =
  "You are one session of a looper loop working on this project. The orientation pack below names the live goal, its live stage, the last session's handoff and the traps. Take the next group of work from the handoff, check it as AGENTS.md § *Session workflow* says, commit it, and overwrite the goal's ## Handoff with where the work stands. Then write one line to {status}: `CONTINUE <what landed>`, `DONE <the goal reached>` or `BLOCKED <what the loop cannot do itself>`. A design call never blocks: decide it by the project's priorities, record it, and build on it. Ask the user nothing you can decide; only a call the project leaves open goes to `looper decision ask --built-on <n>` with your recommendation. BLOCKED is only for a refused permission, a missing credential, or an act that publishes outside the repository or costs money; before it, file that with `looper decision ask --parked`, unless you wait only on decisions already open. Then stop.";

/** The opening prompt of a repair session, which the verdict follows. `{status}` as in `SESSION_PROMPT`. */
export const REPAIR_PROMPT =
  "You are a repair session of a looper loop. The loop stopped making progress for the reason below. Fix exactly that and commit the fix. Then write one line to {status}: `CONTINUE <what you fixed>` or `BLOCKED <what the loop cannot do itself>`, and stop. A design call never blocks: decide it by the project's priorities, record it, and build on it. Ask the user nothing you can decide; only a call the project leaves open goes to `looper decision ask --built-on <n>` with your recommendation. BLOCKED is only for a refused permission, a missing credential, or an act that publishes outside the repository or costs money; before it, file that with `looper decision ask --parked`.";

/** The opening prompt of a landing session, which the verdict follows. `{status}` as in `SESSION_PROMPT`. */
export const LANDING_PROMPT =
  "You are a landing session of a looper loop. This side branch's goal is reached and retired, and merging the main branch into it failed for the reason below. Merge the main branch as the reason says, keep the work of both sides in each conflict, make `looper verify` green, and commit the merge. Then write one line to {status}: `CONTINUE <what you merged>` or `BLOCKED <what the loop cannot do itself>`, and stop. A design call never blocks: decide it by the project's priorities, record it, and build on it. Ask the user nothing you can decide; only a call the project leaves open goes to `looper decision ask --built-on <n>` with your recommendation. BLOCKED is only for a refused permission, a missing credential, or an act that publishes outside the repository or costs money; before it, file that with `looper decision ask --parked`.";

/** The prompt of a decision's advisor, which the decision, its thread and the loop's orientation pack follow. */
export const ADVISOR_PROMPT =
  "You advise the user on one decision a session of a looper loop asked them. The decision, its discussion so far and the loop's orientation pack follow. You may read the tree, and you change nothing. Answer the user's last message in at most 1500 bytes of plain prose: recommend one option and say why, in the terms of the priority ordering in AGENTS.md. When the discussion shows better options or a better recommendation, end your reply with a fenced code block whose info string is `options`, holding `{\"options\": [\"...\", \"...\"], \"recommend\": n}`, with two to four options of at most 200 bytes each and `recommend` numbering one from 1. On a decision the session built on an option, that block holds only `recommend`.";

/** What the session after a `BLOCKED` line that filed no parked decision is told, after its prompt. */
export const UNFILED_BLOCK =
  "## Your BLOCKED filed no decision\n\nThe last session wrote BLOCKED, but filed no parked decision for the user. If the work is still blocked, run `looper decision ask <question> --option <text>… --recommend <n> --parked` first, then write BLOCKED again.";

/** The file in a session's folder that records it. */
export const SESSION_FILE = "session.json";

/** The most checks whose last result a run keeps. */
export const CHECKS_MAX = 1024;

/** The prompts a session opens with. The kit ships defaults; a project may name its own. */
export interface Prompts {
  session: string;
  repair: string;
  landing: string;
}

/**
 * The prompts the next session opens with: each file `[prompts]` names, read from under `root` now, or
 * else the kit's own, with `{status}` as the status file's path. Throws why a named file cannot be read.
 */
export function readPrompts(root: string, named: Named, status: string): Prompts {
  const read = (which: string, path: string | undefined, builtin: string) => {
    if (path === undefined) return builtin;
    try {
      return readFileSync(join(root, path), "utf8").trimEnd();
    } catch (e) {
      throw new Refused(`the ${which} prompt ${path} cannot be read: ${(e as Error).message}`);
    }
  };
  return {
    session: read("session", named.session, SESSION_PROMPT).replaceAll("{status}", status),
    repair: read("repair", named.repair, REPAIR_PROMPT).replaceAll("{status}", status),
    landing: LANDING_PROMPT.replaceAll("{status}", status),
  };
}

/** The waits and limits that are the kit's policy rather than the project's. Every wait is in ms. */
export interface Policy {
  /** The waits after one, two, three ... overloads in a row; the last repeats. */
  overloadBackoff: number[];
  /** The waits before the first, second ... rejoin of a dropped stream; the last repeats. */
  resumeBackoff: number[];
  maxResumes: number;
  /** The wait after the first CLI failure, doubled for each one after it. */
  failureBackoff: number;
  failureBackoffMax: number;
  /** Usage walls in a row that end the run. */
  maxWalls: number;
}

export function defaultPolicy(): Policy {
  return {
    overloadBackoff: [60, 120, 300, 600].map((s) => s * 1000),
    resumeBackoff: [15, 30, 60].map((s) => s * 1000),
    maxResumes: 3,
    failureBackoff: 30_000,
    failureBackoffMax: 300_000,
    maxWalls: 8,
  };
}

/** What the next session is. `retry` retries a `DONE` the checks refused, and `landing` is a side loop's. */
export type Mode = "work" | "retry" | "repair" | "landing";

/** A session whose stream dropped, to rejoin. */
export interface Dropped {
  /** The agent's session id, for `--resume`. */
  agentSession: string;
  /** The paths it wrote, which the rejoined session's sweep also takes. */
  written: Set<string>;
  /** HEAD before it started, so the rejoined session's commits count from there. */
  base: string;
}

/** The session the loop's variables name. */
export interface Running {
  /** Its number within the loop. */
  number: number;
  /** Its folder. */
  log: string;
  /** The commit it started from. */
  base: string;
}

/** A check's last result in the run, and how many times it ran. */
export interface CheckRuns {
  green: boolean;
  runs: number;
}

/** What a run carries from one turn to the next. */
export interface RunState {
  mode: Mode;
  /** What a retry or a repair session is told to fix. */
  verdict: string;
  /** The goal the per-goal counts below belong to. */
  goal: string;
  repairs: number;
  doneClaims: number;
  /** The `DONE` claims refused because a decision for the user stood. */
  decisionClaims: number;
  /** The `BLOCKED` lines in a row that filed no parked decision. */
  blockedClaims: number;
  /** Counts of the same thing in a row. */
  stalls: number;
  failures: number;
  overloads: number;
  walls: number;
  resumes: number;
  /** The verdict of the last hold. */
  held: string | null;
  dropped: Dropped | null;
  /** Says that arrived while no session could take them, for the next opening. */
  notes: string[];
  /** The next fresh session takes the open stage after the live one. */
  skipStage: boolean;
  /** The sessions the run has started, and the most it may start. */
  sessions: number;
  maxSessions: number | null;
  /** The last session started, from its start until the caller clears it after its `after_session` hooks. */
  session: Running | null;
  /** The warnings of hooks that failed with `on_fail = "warn"`, and of `Driver.reload`, until the caller records them. */
  warnings: string[];
  /** The every-session sweeps the run has run. */
  sweeps: number;
  /** A read of `looper.toml` has named another adapter, and its warning is kept. */
  adapterWarned: boolean;
  /** The run's check history: each check's last result, at most `CHECKS_MAX`, the oldest dropped first. */
  checks: Map<string, CheckRuns>;
  /** The retired side goal whose branch is catching up with the main branch. */
  catchingUp: string | null;
}

export function newRunState(): RunState {
  return {
    mode: "work",
    verdict: "",
    goal: "",
    repairs: 0,
    doneClaims: 0,
    decisionClaims: 0,
    blockedClaims: 0,
    stalls: 0,
    failures: 0,
    overloads: 0,
    walls: 0,
    resumes: 0,
    held: null,
    dropped: null,
    notes: [],
    skipStage: false,
    sessions: 0,
    maxSessions: null,
    session: null,
    warnings: [],
    sweeps: 0,
    adapterWarned: false,
    checks: new Map(),
    catchingUp: null,
  };
}

/** Keeps `note` for the next session, within `NOTES_MAX`. */
export function addNote(s: RunState, note: string): void {
  if (s.notes.length < NOTES_MAX) s.notes.push(note);
}

/** Keeps a warning for the caller, within `NOTES_MAX`. */
export function addWarning(s: RunState, warning: string): void {
  if (s.warnings.length < NOTES_MAX) s.warnings.push(warning);
}

/** Starts the per-goal state afresh for `goal`, keeping what the operator asked and the run's counts. */
function reset(s: RunState, goal: string): RunState {
  return {
    ...newRunState(),
    goal,
    notes: s.notes,
    skipStage: s.skipStage,
    sessions: s.sessions,
    maxSessions: s.maxSessions,
    session: s.session,
    warnings: s.warnings,
    sweeps: s.sweeps,
    adapterWarned: s.adapterWarned,
    checks: s.checks,
  };
}

/** What a `set` control asks of the sessions ahead, over `looper.toml`, for the rest of the run. */
export interface Settings {
  model?: string;
  /** An empty effort launches with none. */
  effort?: string;
  budgetHard?: number;
  budgetKill?: number;
  maxSessions?: number;
}

/**
 * Reads a `set` payload: a JSON object with any of `model`, `effort`, `budget.hard`, `budget.kill` and
 * `max_sessions`. Any other key, a value of the wrong type, an empty model, a zero, or a `budget.hard`
 * not below `budget.kill` is refused with why.
 */
export function parseSettings(payload: string): Settings {
  let v: unknown;
  try {
    v = JSON.parse(payload);
  } catch (e) {
    throw new Refused(`a set payload is a JSON object: ${(e as Error).message}`);
  }
  if (typeof v !== "object" || v === null || Array.isArray(v)) throw new Refused("a set payload is a JSON object");
  const text = (k: string, x: unknown) => {
    if (typeof x !== "string") throw new Refused(`\`${k}\` is a string`);
    return x;
  };
  const count = (k: string, x: unknown) => {
    if (typeof x !== "number" || !Number.isSafeInteger(x) || x <= 0) throw new Refused(`\`${k}\` is a whole number above zero`);
    return x;
  };
  const s: Settings = {};
  for (const [k, x] of Object.entries(v)) {
    if (k === "model") s.model = text(k, x);
    else if (k === "effort") s.effort = text(k, x);
    else if (k === "budget.hard") s.budgetHard = count(k, x);
    else if (k === "budget.kill") s.budgetKill = count(k, x);
    else if (k === "max_sessions") {
      s.maxSessions = count(k, x);
      if (s.maxSessions > 0xffff_ffff) throw new Refused("`max_sessions` is too big");
    } else throw new Refused(`set takes model, effort, budget.hard, budget.kill and max_sessions, not \`${k}\``);
  }
  if (s.model === "") throw new Refused("`model` names a model");
  if (s.budgetHard !== undefined && s.budgetKill !== undefined && s.budgetHard >= s.budgetKill) {
    throw new Refused("`budget.hard` is below `budget.kill`");
  }
  return s;
}

/** How a turn ended, and so what the caller does next. */
export type Outcome =
  /** The goal was reached and retired. */
  | { kind: "advanced"; slug: string; said: string }
  /** Run the next turn. */
  | { kind: "continue" }
  /** Run the next turn at `until` (ms since the Unix epoch), or sooner on `retry-now`. */
  | { kind: "wait"; until: number; why: string }
  /** Hold for a person: an agent-owned pause with this reason. */
  | { kind: "hold"; why: string }
  /** End the run. */
  | { kind: "end"; why: string }
  /** The retired side goal's branch has caught up with the main branch, and the caller lands it. */
  | { kind: "land"; slug: string; said: string };

/** What one session's `session.json` holds. */
export interface SessionRecord {
  number: number;
  goal: string;
  stage: number | null;
  /** `work`, `retry`, `repair`, `landing` or `resume`. */
  kind: string;
  /** In ms since the Unix epoch. */
  started: number;
  /** Null while it runs. */
  ended: number | null;
  /** How the session ended, and what the turn decided, once it has ended. */
  exit: string;
  statusLine: string | null;
  verdict: string;
  commits: number;
  peakContext: number;
  firstContext: number;
  toolCalls: number;
  tokensIn: number;
  tokensOut: number;
  tokensCacheRead: number;
  tokensCacheWrite: number;
  costUsd: number;
}

/** Everything a driver starts with. Each field it leaves out takes its default. */
export interface DriverOptions {
  /** The loop's worktree. */
  root: string;
  /** The loop's folder, `.looper/loops/<name>/`, which gets `sessions/`. */
  dir: string;
  adapter: AgentAdapter;
  /** The usage walls this loop reads and notes. */
  walls: Walls;
  /** The account the agent runs on, which its usage walls belong to. */
  account: string;
  /** Where a session writes its status line, outside the project's tree. */
  statusFile: string;
  /** Where each check's last output is kept. */
  checkLogs: string;
  /** Where each hook's last output is kept. */
  hookLogs: string;
  loopName?: string;
  loopKind?: string;
  cfg?: Config;
  prompts?: Prompts;
  policy?: Policy;
  limits?: Limits;
  live?: (e: Live) => void;
  stats?: (s: Stats | null) => void;
  progress?: Progress;
  /** A side loop's goal and landing places, left out for a main loop and a side loop in the project root. */
  side?: Side;
  /** The goal of a side loop in the project root, left out for every other loop. */
  ownGoal?: string;
}

/** A session that has ended, and what it left. */
interface Ended {
  record: SessionRecord;
  /** Its folder. */
  dir: string;
  served: Served;
  /** HEAD before the session, or before the session it rejoined. */
  base: string;
  status: string | null;
}

/** The `n`th entry of `list`, from 1, with the last one repeating. */
function nth(list: number[], n: number): number {
  return list[Math.min(Math.max(n, 1) - 1, list.length - 1)] ?? 0;
}

/** One loop's turns. */
export class Driver {
  root: string;
  dir: string;
  loopName: string;
  loopKind: string;
  cfg: Config;
  adapter: AgentAdapter;
  walls: Walls;
  account: string;
  prompts: Prompts;
  policy: Policy;
  limits: Limits;
  /** The run's `set` controls, merged key by key, which win over every read of `looper.toml`. */
  settings: Settings = {};
  live: ((e: Live) => void) | undefined;
  stats: ((s: Stats | null) => void) | undefined;
  statusFile: string;
  checkLogs: string;
  hookLogs: string;
  progress: Progress;
  /** A side loop's goal and landing places, and null for a main loop. */
  side: Side | null;
  /** The goal of a side loop in the project root, and null for every other loop. */
  ownGoal: string | null;
  state: RunState = newRunState();
  /** The number the loop's next session takes, read from `sessions/` at the first session. */
  private next: number | null = null;

  constructor(o: DriverOptions) {
    this.root = o.root;
    this.dir = o.dir;
    this.loopName = o.loopName ?? "main";
    this.loopKind = o.loopKind ?? "main";
    this.cfg = o.cfg ?? defaultConfig();
    this.adapter = o.adapter;
    this.walls = o.walls;
    this.account = o.account;
    this.prompts = o.prompts ?? readPrompts(o.root, this.cfg.prompts, o.statusFile);
    this.policy = o.policy ?? defaultPolicy();
    this.limits = o.limits ?? limitsFrom(this.cfg);
    this.live = o.live;
    this.stats = o.stats;
    this.statusFile = o.statusFile;
    this.checkLogs = o.checkLogs;
    this.hookLogs = o.hookLogs;
    this.progress = o.progress ?? unreported;
    this.side = o.side ?? null;
    this.ownGoal = o.ownGoal ?? null;
  }

  /** The prompt the next session opens with, before the pack. */
  prompt(): string {
    const s = this.state;
    let prompt: string;
    switch (s.mode) {
      case "work":
        prompt = this.prompts.session;
        break;
      case "retry":
        prompt = `${this.prompts.session}\n## Your DONE was refused\n\nThe last session wrote DONE, but acceptance did not reach the goal. Fix exactly this, then claim DONE again:\n\n${s.verdict}\n`;
        break;
      case "repair":
        prompt = `${this.prompts.repair}\n\`\`\`\n${s.verdict}\n\`\`\`\n`;
        break;
      case "landing":
        prompt = `${this.prompts.landing}\n\`\`\`\n${s.verdict}\n\`\`\`\n`;
        break;
    }
    if (s.blockedClaims > 0) prompt += `\n${UNFILED_BLOCK}\n`;
    if (s.notes.length > 0) {
      prompt += "\nNotes from the operator, given while no session could receive them:\n";
      for (const n of s.notes) prompt += `- ${n}\n`;
    }
    return prompt;
  }

  /** Step 1: what stops the turn before a session starts. */
  private gate(): Outcome | null {
    const now = Date.now();
    const w = this.walls.standing(this.account, now);
    if (w === null) return null;
    // A wall further out than the maximum wait is read again after it, so a lift elsewhere reaches the loop.
    const until = Math.min(clearsAt(w), now + this.cfg.loop.max_limit_wait_s * 1000);
    return { kind: "wait", until, why: `the ${w.kind === "" ? "usage" : w.kind} wall on ${this.account}` };
  }

  /** Lays what a `set` control asked over the sessions ahead, for the rest of the run, merged into the run's `settings`. */
  set(s: Settings): void {
    for (const [k, v] of Object.entries(s)) if (v !== undefined) (this.settings as Record<string, unknown>)[k] = v;
    this.lay();
  }

  /** Lays the run's `settings` over `cfg`, `limits` and the run's session cap. */
  private lay(): void {
    const s = this.settings;
    if (s.model !== undefined) this.cfg.agent.model = s.model;
    if (s.effort !== undefined) this.cfg.agent.effort = s.effort;
    if (s.budgetHard !== undefined) this.limits.hard = s.budgetHard;
    if (s.budgetKill !== undefined) this.limits.kill = s.budgetKill;
    if (s.maxSessions !== undefined) this.state.maxSessions = s.maxSessions;
  }

  /**
   * Reads the worktree's `looper.toml` again into `cfg` and `limits`, and lays the run's `settings` over
   * them. The adapter stays as the run started: a read that names another one keeps a warning, once per
   * run. A file that does not load throws, and leaves both as they were.
   */
  reload(): void {
    const cfg = loadConfig(this.root);
    if (cfg.agent.adapter !== this.cfg.agent.adapter) {
      if (!this.state.adapterWarned) {
        this.state.adapterWarned = true;
        addWarning(this.state, `looper.toml names the adapter "${cfg.agent.adapter}", which starts with the loop's next run`);
      }
      cfg.agent.adapter = this.cfg.agent.adapter;
    }
    this.limits = { ...limitsFrom(cfg), backgroundWait: this.limits.backgroundWait, followUpGrace: this.limits.followUpGrace };
    this.cfg = cfg;
    this.lay();
  }

  /** The project's unit store, which keeps the units of command checks. */
  private units(): Units {
    return { store: storeDir(this.root), all: false };
  }

  private goalsDir(): string {
    return join(this.root, this.cfg.project.goals_dir);
  }

  /**
   * The loop's variables now: the goal's (`goalVars`), the loop's, the run's, and the last session's
   * while `RunState.session` holds it. One with no value is `undefined`.
   */
  vars(): Record<string, string | undefined> {
    const s = this.state.session;
    return {
      ...goalVars(this.root, this.goalsDir()),
      LOOPER_LOOP: this.loopName,
      LOOPER_LOOP_KIND: this.loopKind,
      [SIDE_GOAL_VAR]: this.ownGoal ?? undefined,
      LOOPER_RUN_SESSIONS: String(this.state.sessions),
      LOOPER_SESSION: s?.number.toString(),
      LOOPER_SESSION_LOG: s?.log,
      LOOPER_SESSION_BASE: s?.base,
    };
  }

  /** The project's `[checks]` with the loop's variables. */
  private project(): Project {
    return { checks: this.cfg.checks, vars: this.vars() };
  }

  /** Runs `event`'s hooks with the loop's variables, keeps their warnings, and gives why the hook that pauses the loop failed. */
  async hooks(event: HookEvent): Promise<string | null> {
    return this.hooksWith(event, this.project());
  }

  /** Runs `event`'s hooks with `project`, and keeps their warnings as `hooks` does. */
  private async hooksWith(event: HookEvent, project: Project): Promise<string | null> {
    const ran = await runHooks(this.root, event, this.cfg.hooks, this.hookLogs, project);
    for (const w of ran.warnings) addWarning(this.state, w);
    return ran.paused;
  }

  /** Runs the `after_retire` hooks with `LOOPER_GOAL` naming the retired `slug`, and no `LOOPER_STAGE` or `LOOPER_EXTRA`. */
  private async afterRetire(slug: string): Promise<string | null> {
    const project = this.project();
    project.vars = { ...project.vars, LOOPER_GOAL: slug, LOOPER_STAGE: undefined, LOOPER_EXTRA: undefined };
    return this.hooksWith("after_retire", project);
  }

  /** Adds `verdicts` to the run's check history, dropping the oldest past `CHECKS_MAX`. */
  private addCheckRuns(verdicts: Verdict[]): void {
    const history = this.state.checks;
    for (const v of verdicts) {
      const runs = (history.get(v.id)?.runs ?? 0) + 1;
      history.delete(v.id);
      history.set(v.id, { green: v.green, runs });
      if (history.size > CHECKS_MAX) history.delete(history.keys().next().value as string);
    }
  }

  /**
   * Runs the `before_sweep` hooks, then the live goal's acceptance between sessions as `check` does, or
   * with a non-empty `name` only the checks it matches, and adds the results to the run's check history.
   * A hook that pauses the loop refuses the check with its reason.
   */
  async check(name: string): Promise<Verdict[]> {
    const goalsDir = this.goalsDir();
    const chain = loadChain(goalsDir);
    const slug = liveGoal(this.root, goalsDir, chain);
    if (slug === undefined) return [];
    const goal = loadGoal(goalsDir, slug);
    const why = await this.hooks("before_sweep");
    if (why !== null) throw new Refused(why);
    const verdicts =
      name === ""
        ? (await acceptance(this.root, goal, chain.floor, this.checkLogs, this.units(), this.project(), this.progress)).verdicts
        : await runReported(this.root, named(chain.floor, goal, name), this.checkLogs, this.units(), this.project(), this.progress, []);
    this.addCheckRuns(verdicts);
    return verdicts;
  }

  /** Runs one turn. `controls` reach the session while it runs. */
  async turn(controls?: Controls): Promise<Outcome> {
    const catching = this.state.catchingUp;
    if (catching !== null && this.state.mode !== "landing") return this.catchUp(catching);
    await this.walls.pull(this.account, Date.now());
    const stop = this.gate();
    if (stop !== null) return stop;
    const max = this.state.maxSessions;
    if (max !== null && this.state.sessions >= max) {
      return { kind: "end", why: `the run has run the ${max} sessions \`set\` allows it` };
    }
    if (catching !== null) {
      const dropped = this.state.dropped;
      this.state.dropped = null;
      const opening = dropped !== null ? this.adapter.inject(RESUME_PROMPT) : this.adapter.opening(this.prompt(), "");
      this.state.notes = [];
      return this.serveTurn(catching, null, dropped, opening, controls);
    }

    // 2. Pick.
    const goalsDir = this.goalsDir();
    const own = this.ownGoal;
    if (own !== null && !existsSync(goalPath(goalsDir, own))) {
      return { kind: "end", why: `${own} has no goal file in ${this.root}: it is retired, so this side loop is done` };
    }
    const chain = loadChain(goalsDir);
    const slug = liveGoal(this.root, goalsDir, chain);
    if (this.side !== null && slug !== this.side.goal) {
      const g = this.side.goal;
      return this.hold(
        `this side loop works ${g}, but the live goal in ${this.root} is ${slug ?? "none"}: the checkout must be on side/${g} with the goal's file`,
      );
    }
    if (own !== null && slug !== own) {
      return this.hold(`this side loop works ${own}, but the live goal in ${this.root} is ${slug ?? "none"}: its worker must run with ${SIDE_GOAL_VAR}=${own}`);
    }
    if (slug === undefined) return { kind: "end", why: "the chain is complete: there is no live goal" };
    if (this.state.goal !== slug) this.state = reset(this.state, slug);
    const goal = loadGoal(goalsDir, slug);
    const dropped = this.state.dropped;
    this.state.dropped = null;
    const skip = dropped === null && this.state.skipStage;
    if (dropped === null) this.state.skipStage = false;
    const open = goal.front.stages.filter((s) => s.status !== "done");
    const live = open[0];
    const stage = skip ? (open[1] ?? live) : live;

    // 3. Open.
    let opening: string;
    if (dropped !== null) {
      opening = this.adapter.inject(RESUME_PROMPT);
    } else {
      let prompt = this.prompt();
      if (live !== undefined && stage !== undefined && live.id !== stage.id) {
        prompt += `\nThe operator skipped stage ${live.id} for this session: take stage ${stage.id}, which the pack below shows as the live stage, and leave stage ${live.id}'s status as it is.\n`;
      }
      const sections = await packExtra(this.root, this.cfg, stage?.id, false, this.project().vars);
      const decided = packSection(this.dir);
      if (decided !== null) sections.push(decided);
      opening = this.adapter.opening(prompt, pack(this.root, this.cfg, false, stage?.id, sections));
    }
    this.state.notes = [];
    return this.serveTurn(slug, stage?.id ?? null, dropped, opening, controls);
  }

  /** Steps 4 to 8 of a turn on `slug`: serves the session that opens with `opening`, records its end, and decides. */
  private async serveTurn(slug: string, stage: number | null, dropped: Dropped | null, opening: string, controls?: Controls): Promise<Outcome> {
    const e = await this.serveOne(slug, stage, dropped, opening, controls);
    const now = Date.now();
    const cls: ExitClass = e.served.killed !== null ? { kind: "ok" } : this.adapter.classify(sessionEnd(e.served, Math.floor(now / 1000)));
    const r = e.record;
    r.ended = now;
    r.exit = exitText(e.served, cls);
    r.statusLine = e.status;
    r.peakContext = e.served.peakContext;
    r.firstContext = e.served.firstContext;
    r.toolCalls = e.served.toolCalls;
    const result = e.served.result;
    if (result !== null) {
      r.tokensIn = result.usage.input;
      r.tokensOut = result.usage.output;
      r.tokensCacheRead = result.usage.cacheRead;
      r.tokensCacheWrite = result.usage.cacheWrite;
      r.costUsd = result.costUsd;
    }
    try {
      this.reload();
    } catch (err) {
      addWarning(this.state, `looper.toml does not load after the session, so the turn keeps its config: ${(err as Error).message}`);
    }
    const outcome = await this.afterExit(cls, e, slug, now);
    r.verdict = verdictText(outcome, this.state.mode);
    writeState(join(e.dir, SESSION_FILE), r);
    return outcome;
  }

  /** The loop's next session number: one past the highest folder under `sessions/`, read once a run. */
  private nextNumber(): number {
    if (this.next === null) {
      const dir = join(this.dir, "sessions");
      const taken = existsSync(dir) ? readdirSync(dir).map(Number).filter(Number.isSafeInteger) : [];
      this.next = taken.reduce((a, b) => Math.max(a, b), 0) + 1;
    }
    return this.next++;
  }

  /** Step 4: records the session's start, serves it into its folder, and takes the status line. */
  private async serveOne(slug: string, stage: number | null, dropped: Dropped | null, opening: string, controls?: Controls): Promise<Ended> {
    const number = this.nextNumber();
    const dir = join(this.dir, "sessions", String(number));
    mkdirSync(dir, { recursive: true });
    const base = dropped?.base ?? git(this.root, ["rev-parse", "HEAD"]).trim();
    const record: SessionRecord = {
      number,
      goal: slug,
      stage,
      kind: dropped !== null ? "resume" : this.state.mode,
      started: Date.now(),
      ended: null,
      exit: "",
      statusLine: null,
      verdict: "",
      commits: 0,
      peakContext: 0,
      firstContext: 0,
      toolCalls: 0,
      tokensIn: 0,
      tokensOut: 0,
      tokensCacheRead: 0,
      tokensCacheWrite: 0,
      costUsd: 0,
    };
    writeState(join(dir, SESSION_FILE), record);
    this.state.sessions++;
    const writtenList = join(dir, WRITTEN_LIST);
    rmSync(writtenList, { force: true });
    rmSync(this.statusFile, { force: true });
    this.state.session = { number, log: dir, base };
    const served = await serve({
      adapter: this.adapter,
      opts: {
        cwd: this.root,
        model: this.cfg.agent.model,
        permissionMode: this.cfg.agent.permission_mode,
        tools: this.cfg.agent.tools,
        effort: this.cfg.agent.effort === "" ? undefined : this.cfg.agent.effort,
        resume: dropped?.agentSession,
        written: writtenList,
        loop: this.loopName,
        root: resolve(this.dir, "..", "..", ".."),
        goal: slug,
        sideGoal: this.ownGoal ?? undefined,
      },
      opening,
      limits: { ...this.limits },
      dir,
      live: this.live,
      stats: this.stats,
      goal: join(this.goalsDir(), `${slug}.md`),
      controls,
    });
    this.releaseClaims(served.agentSession);
    rmSync(writtenList, { force: true });
    for (const n of served.notes) addNote(this.state, n);
    served.notes = [];
    for (const p of dropped?.written ?? []) {
      if (served.written.size >= WRITTEN_MAX) break;
      served.written.add(p);
    }
    let status: string | null = null;
    try {
      status = readFileSync(this.statusFile, "utf8").split("\n")[0]?.trim() || null;
    } catch {}
    rmSync(this.statusFile, { force: true });
    return { record, dir, served, base, status };
  }

  /** Frees the edit claims of the agent session `agentSession`, which has ended. A failure is a warning. */
  private releaseClaims(agentSession: string): void {
    if (agentSession === "") return;
    try {
      release(claimDir(this.root), agentSession);
    } catch (e) {
      addWarning(this.state, `cannot release the edit claims of session ${agentSession}: ${(e as Error).message}`);
    }
  }

  /** Steps 5 to 8: what the exit class `cls` of the session `e` of `slug` makes of the run. */
  private async afterExit(cls: ExitClass, e: Ended, slug: string, now: number): Promise<Outcome> {
    const { served, status } = e;
    const index = e.record.number;
    const s = this.state;
    if (cls.kind !== "wall") s.walls = 0;
    if (cls.kind !== "overload") s.overloads = 0;
    if (cls.kind !== "dropped") s.resumes = 0;

    if (cls.kind === "wall") {
      const w = this.walls.note(this.account, cls.resetsAt, served.limit?.kind ?? "", now);
      await this.walls.report(this.account, w);
      if (this.sweep(index, "the account hit its usage limit", served.written, status)) return this.hold(MERGE_HOLD);
      s.walls++;
      if (s.walls >= this.policy.maxWalls) return { kind: "end", why: `${s.walls} usage walls in a row` };
      return { kind: "wait", until: clearsAt(w), why: `the usage wall on ${this.account}` };
    }
    if (cls.kind === "overload") {
      if (this.sweep(index, "the API was overloaded", served.written, status)) return this.hold(MERGE_HOLD);
      s.overloads++;
      return { kind: "wait", until: now + nth(this.policy.overloadBackoff, s.overloads), why: `the API was overloaded (${s.overloads} in a row)` };
    }
    if (cls.kind === "dropped" && s.resumes < this.policy.maxResumes && served.agentSession !== "") {
      s.resumes++;
      s.dropped = { agentSession: served.agentSession, written: served.written, base: e.base };
      return {
        kind: "wait",
        until: now + nth(this.policy.resumeBackoff, s.resumes),
        why: `the stream dropped; rejoining it (${s.resumes}/${this.policy.maxResumes})`,
      };
    }
    if ((cls.kind === "dropped" || cls.kind === "failed") && status === null) {
      const why = `the CLI exited ${codeText(served.code)}`;
      if (this.sweep(index, why, served.written, status)) return this.hold(MERGE_HOLD);
      s.failures++;
      if (s.failures >= this.cfg.loop.max_retries) {
        s.failures = 0;
        const last = served.result?.text ?? "";
        return this.repairOrHold(`cli-failed: ${why}, ${this.cfg.loop.max_retries} times in a row. Last result: ${last}`);
      }
      const doubled = this.policy.failureBackoff * 2 ** Math.min(s.failures, 16);
      return { kind: "wait", until: now + Math.min(doubled, this.policy.failureBackoffMax), why: `${why}; backing off` };
    }
    s.failures = 0;
    const merging = this.sweep(index, served.killed ?? "it ended without committing these", served.written, status);
    const commits = Number.parseInt(git(this.root, ["rev-list", "--count", `${e.base}..HEAD`]).trim(), 10) || 0;
    e.record.commits = commits;
    // A catch-up finishes a merge a landing session left, or holds on its conflicts.
    if (s.catchingUp !== null) {
      if (status?.startsWith("BLOCKED")) return this.blocked("the landing session", status, e.record.started);
      s.blockedClaims = 0;
      return this.catchUp(slug);
    }
    if (merging) return this.hold(MERGE_HOLD);
    const outcome = await this.judge(slug, status, commits, e.record.started);
    const claimed = status?.startsWith("DONE") ?? false;
    return this.cfg.loop.sweep === "every" && !claimed ? this.sweepEvery(slug, outcome) : outcome;
  }

  /**
   * Step 6: sweeps the session `index`, which ended as `why`. Then `.loop/interrupted.json` names its
   * sweep commit when it wrote no status line, or goes when it wrote one. Gives whether a merge was
   * under way, so the sweep changed nothing.
   */
  private sweep(index: number, why: string, written: Set<string>, status: string | null): boolean {
    const swept = sweep(this.root, index, why, written);
    if (status !== null) wrapped(this.root);
    else interrupted(this.root, index, swept, why);
    return swept.merging;
  }

  /**
   * Step 9, the every-session sweep after a session judged to `outcome` that did not claim `DONE`. It
   * runs the `before_sweep` hooks, then acceptance with each check's `every`, records the checks it ran,
   * and writes `.loop/last-sweep.md`. It never retires a goal. A hook that pauses the loop turns a
   * `continue` into a hold and skips the sweep.
   */
  private async sweepEvery(slug: string, outcome: Outcome): Promise<Outcome> {
    const goalsDir = this.goalsDir();
    let chain: ReturnType<typeof loadChain>;
    let goal: Goal;
    try {
      chain = loadChain(goalsDir);
      goal = loadGoal(goalsDir, slug);
    } catch {
      return outcome;
    }
    const why = await this.hooks("before_sweep");
    if (why !== null) return outcome.kind === "continue" ? this.hold(setupHold(why)) : outcome;
    this.state.sweeps++;
    const counted = this.notDue(chain.floor, goal);
    const acc = await acceptance(this.root, goal, chain.floor, this.checkLogs, this.units(), this.project(), this.progress, counted);
    this.addCheckRuns(acc.verdicts.filter((v) => !counted.has(v.id)));
    lastSweep(this.root, acc.verdicts);
    return outcome;
  }

  /**
   * The checks of `floor` and `goal` this sweep counts rather than runs: each with an `every` this sweep
   * is not a multiple of, mapped to its last result. One the history holds no result for runs.
   */
  private notDue(floor: Check[], goal: Goal): Map<string, boolean> {
    const n = this.state.sweeps;
    const counted = new Map<string, boolean>();
    for (const c of [...floor, ...goal.front.checks]) {
      const last = this.state.checks.get(c.id);
      if (c.every !== undefined && c.every > 1 && n % c.every !== 0 && last !== undefined) counted.set(c.id, last.green);
    }
    return counted;
  }

  /** Steps 7 and 8 for a session that started at `started`, ended with `commits` since its base and wrote `status`. */
  private async judge(slug: string, statusLine: string | null, commits: number, started: number): Promise<Outcome> {
    const goalsDir = this.goalsDir();
    let chain: ReturnType<typeof loadChain>;
    let goal: Goal;
    try {
      chain = loadChain(goalsDir);
      goal = loadGoal(goalsDir, slug);
    } catch (e) {
      return this.repairOrHold(`chain-error: ${(e as Error).message}`);
    }
    const status = statusLine ?? "";
    const s = this.state;
    let why: string[] = [];
    const waits = status.startsWith("DONE") ? standing(this.dir) : null;
    if (status.startsWith("DONE") && waits === null) {
      const paused = await this.hooks("before_sweep");
      if (paused !== null) return this.hold(setupHold(paused));
      const acc = await acceptance(this.root, goal, chain.floor, this.checkLogs, this.units(), this.project(), this.progress);
      this.addCheckRuns(acc.verdicts);
      if (this.cfg.loop.sweep === "every") lastSweep(this.root, acc.verdicts);
      why = notReached(this.root, this.cfg, goal, acc);
      if (why.length === 0 && goal.front.stages.length > 0) {
        let said: string;
        try {
          said = retire(this.root, this.cfg, slug);
        } catch (e) {
          return this.hold(`retire-error: ${slug} is reached, but retiring it failed: ${(e as Error).message}`);
        }
        this.state = reset(this.state, "");
        if (this.side !== null) this.state.catchingUp = slug;
        const failed = await this.afterRetire(slug);
        if (failed !== null) return this.hold(setupHold(`${slug} is retired, but ${failed}`));
        if (this.side !== null) return this.catchUp(slug);
        return { kind: "advanced", slug, said };
      }
    }
    s.stalls = commits === 0 ? s.stalls + 1 : 0;
    if (status.startsWith("BLOCKED")) return this.blocked("the session", status, started);
    s.blockedClaims = 0;
    if (waits !== null) {
      s.decisionClaims++;
      // Only the user can answer an open decision, so the loop holds on each one at once.
      let held: string[] = [];
      try {
        held = (await block(this.dir, readDecisions(this.dir).decisions.map((x) => x.id))).map((x) => x.id);
      } catch {}
      if (s.decisionClaims > 1 || held.length > 0) {
        s.decisionClaims = 0;
        s.mode = "work";
        s.verdict = "";
        return this.hold(`decisions: ${waits}`);
      }
      s.mode = "retry";
      s.verdict = `- ${waits}`;
      s.held = null;
      return { kind: "continue" };
    }
    s.decisionClaims = 0;
    if (status.startsWith("DONE")) {
      s.doneClaims++;
      const reasons = why.map((w) => `- ${w}`).join("\n");
      if (s.doneClaims > this.cfg.loop.done_retries) {
        return this.repairOrHold(`done-claim: ${s.doneClaims} DONE claims for ${slug} were refused.\n${reasons}`);
      }
      s.mode = "retry";
      s.verdict = reasons;
      s.held = null;
      return { kind: "continue" };
    }
    if (s.stalls >= this.cfg.loop.max_stalls) {
      s.stalls = 0;
      return this.repairOrHold(`stalled: ${this.cfg.loop.max_stalls} sessions in a row made no commit on ${slug}.`);
    }
    s.mode = "work";
    s.verdict = "";
    s.held = null;
    return { kind: "continue" };
  }

  /**
   * The catch-up of the retired side goal `slug` (`catchUp` in `catch-up.ts`). A green one returns `land`
   * for the caller to land the branch. The caller runs it again when the main branch moves during a
   * landing.
   */
  async catchUp(slug: string): Promise<Outcome> {
    if (this.side === null) return { kind: "end", why: `${slug} is retired, but only a side loop catches up` };
    // A catch-up the landing runs again comes after a green one cleared this, and a landing session it
    // queues must still be one.
    this.state.catchingUp = slug;
    const caught = await catchUp({
      root: this.root,
      cfg: this.cfg,
      side: this.side,
      slug,
      checkLogs: this.checkLogs,
      units: this.units(),
      project: this.project(),
      progress: this.progress,
    });
    switch (caught.kind) {
      case "hold":
        return this.hold(caught.why);
      case "landing":
        return this.repairOrHold(caught.verdict);
      case "land":
        this.state.mode = "work";
        this.state.verdict = "";
        this.state.catchingUp = null;
        return { kind: "land", slug, said: caught.said };
    }
  }

  /**
   * Queues a repair session handed `verdict`, or a landing session during a catch-up, or holds when the
   * goal has none left or the session that asks was itself one.
   */
  private repairOrHold(verdict: string): Outcome {
    const s = this.state;
    const mode: Mode = s.catchingUp !== null ? "landing" : "repair";
    if (s.mode !== mode && s.repairs < this.cfg.loop.repairs_per_goal) {
      s.repairs++;
      s.mode = mode;
      s.verdict = verdict;
      return { kind: "continue" };
    }
    s.mode = "work";
    s.verdict = "";
    return this.hold(verdict.split("\n")[0] ?? "");
  }

  /**
   * Step 8 for the `BLOCKED` line `status` of `who`, a session that started at `started`. Each open parked
   * decision it asked becomes a blocker, and the loop holds on their ids. A session that asked none is
   * blocked on the decisions already open, so those become the blockers. A line with no open decision
   * at all is retried once in the same mode, told to file one, and the second in a row holds on the
   * reason alone.
   */
  private async blocked(who: string, status: string, started: number): Promise<Outcome> {
    const s = this.state;
    let ids: string[] = [];
    try {
      const open = readDecisions(this.dir).decisions.filter((x) => x.answer === null);
      const asked = open.filter((x) => x.builtOn === null && x.at >= started);
      const on = asked.length > 0 ? asked : open;
      if (on.length > 0) ids = (await block(this.dir, on.map((x) => x.id))).map((x) => x.id);
    } catch {}
    if (ids.length === 0 && s.blockedClaims === 0) {
      s.blockedClaims = 1;
      s.held = null;
      return { kind: "continue" };
    }
    s.blockedClaims = 0;
    s.mode = "work";
    s.verdict = "";
    const on = ids.length === 0 ? "" : ` on decision${ids.length > 1 ? "s" : ""} ${ids.join(", ")}`;
    return this.hold(`${who} is BLOCKED${on}: ${status.slice("BLOCKED".length).trim()}`);
  }

  /** Holds with `why`, or ends the run when the last hold had the same verdict. */
  private hold(why: string): Outcome {
    if (this.state.held === why) {
      this.state.held = null;
      return { kind: "end", why: `the same verdict twice across a hold: ${why}` };
    }
    this.state.held = why;
    return { kind: "hold", why };
  }
}

/** The reason of the hold after a session that ended inside a merge, outside a catch-up. */
export const MERGE_HOLD =
  "merge-under-way: the session ended inside a merge, so looper swept none of its paths. Resolve and commit the merge in this worktree, then resume.";

/** The start of the reason of a hold whose cause a new build may clear: a hook, a prompt file or `looper.toml` that failed. */
export const SETUP_HOLD = "setup: ";

/** `why` as the reason of a setup hold. */
export function setupHold(why: string): string {
  return `${SETUP_HOLD}${why}`;
}

function codeText(code: number | null): string {
  return code === null ? "without an exit code" : String(code);
}

/** The session record's `exit`: how the session ended. */
function exitText(s: Served, cls: ExitClass): string {
  if (s.killed !== null) return `killed: ${s.killed}`;
  const code = codeText(s.code);
  switch (cls.kind) {
    case "ok":
      return `exited ${code}`;
    case "wall":
      return `usage wall, exited ${code}`;
    case "overload":
      return `overloaded, exited ${code}`;
    case "dropped":
      return `stream dropped, exited ${code}`;
    case "failed":
      return `failed, exited ${code}`;
  }
}

/** The session record's `verdict`: what the turn decided. */
function verdictText(o: Outcome, mode: Mode): string {
  switch (o.kind) {
    case "advanced":
      return "advance";
    case "land":
      return "land";
    case "continue":
      return mode === "work" ? "continue" : mode;
    case "wait":
      return `wait: ${o.why}`;
    case "hold":
      return `hold: ${o.why}`;
    case "end":
      return `end: ${o.why}`;
  }
}
