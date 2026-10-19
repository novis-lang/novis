// The worker: one loop's turns in a process of its own, until a turn ends the run or a control stops it.
// It ports `crates/looper/src/cmd/worker.rs`, with the loop's folder where looper has a store and a named
// pipe.
//
// # How it works
//
// - It takes the loop's named mutex for its whole run, so two workers never run one loop. One that finds
//   it held throws `Taken`.
// - It starts paused when a pause stands in `controls.ndjson` (`standingPause` in `controls.ts`), under
//   that pause's owner. So a loop a person paused stays paused when its worker starts again, and a
//   `start` always starts it running.
// - It acts on the controls appended after the last `start` and past the offset the last run's
//   `state.json` read to. It reads new ones every `POLL_MS`, before each turn and while a session runs.
// - The run calls `Driver.turn` until a turn ends it or a control stops it. A `wait` is waited out, until
//   a pause or a stop ends it sooner, and a `hold` pauses the loop until someone resumes it.
// - A side loop's `land` lands its branch as `landing.ts` says, under the state `landing`. A landed loop
//   ends the run. A hold from the landing pauses the loop, and a resume catches up again. A worker that
//   starts while its loop holds a landing goes on with it before any session. One whose goal is retired
//   already catches up first.
// - Before each turn, in this order, it compacts the controls file when it is due (`compact`), and reads
//   `looper.toml` again (`Driver.reload`) under the run's `set` controls. One that does not load holds
//   the loop with its error, and a resume reads it again. Then it lays the `set` and `skip-stage`
//   controls over the sessions ahead and runs a `check` control; holds while the loop is paused; waits
//   while the worktree's volume has less than `[loop] min_free_gb` GiB free, reading it again every
//   `DISK_RECHECK`; reads the prompts; runs the `before_run` hooks, until they pass once in the run; and
//   runs the `before_session` hooks.
// - After a turn that started a session, it runs the `after_session` hooks, and then the loop's
//   variables name no session. One that fails holds the turn.
// - A hook that fails pauses the loop, unless it is `on_fail = "warn"`. A warning is recorded as a `warn`
//   control by `looper` whose payload is the warning, and the loop goes on.
// - A pause the worker makes itself, for a hold or a failed hook, is recorded as a `pause` control by
//   `looper` whose payload is the reason. The reason of a hold whose cause a fixed setup may clear starts
//   with `SETUP_HOLD`.
// - A pause's owner is who asked for it, or `looper` for its own. A person who asks for a pause that
//   holds already takes it over (`pauseOwner`). A `resume` that cannot lift it (`canLift`) leaves the
//   loop paused and is recorded as a `warn` control by `looper`. `appendControl` refuses that resume
//   first, so the worker's check matters only for a line written past it.
// - When the run ends, the worker writes `idle` with why, and releases the mutex.
//
// `state.json` in the loop's folder is the worker's alone, a state file rewritten whenever what it holds
// changes (`WorkerState`). It names the state: `running`, `checking`, `waiting`, `paused`, `landing`, or
// `idle` once the run ended. Beside it is the ask the run has yet to act on: a `stop` or `stop-now` until the run
// ends, or else a `pause` until the loop is paused, each with when it arrived and who asked.
//
// | Control | What the worker does |
// |---|---|
// | `start` | nothing: it lifts a standing pause for the next run (`standingPause`) |
// | `stop` | ends the run once the running session has ended |
// | `stop-now` | kills the running session, then ends the run |
// | `pause` / `resume` | holds before the next session / lets it start, unless an agent asks to lift a person's pause |
// | `retry-now` | ends a wait and lifts the account's usage wall |
// | `skip-stage` | the next fresh session takes the open stage after the live one (`Driver.turn`) |
// | `set` | lays its settings over every session after the running one, merged key by key (`Driver.set`) |
// | `check` | runs acceptance, or with a payload the checks it names, as soon as no session runs, even while paused or waiting (`Driver.check`), and keeps their results in `state.json` |
// | `say`, `interrupt`, `freeze`, `thaw` | reach the running session; between sessions a `say` or an `interrupt` becomes a note for the next one, and the others do nothing |
//
// A session opens with the driver's prompts, or with the file `[prompts] session` or `repair` names,
// read before each turn. Each `{status}` becomes `status.txt` in the loop's folder. Check logs go to its
// `checks/` and hook logs to its `hooks/`.
//
// # The bounds it keeps
//
// - Controls for the running session wait in the session's queue of `CONTROL_BOUND`. One that finds it
//   full is dropped.
// - The asks are single values: a second `set` merges into the first, and a second `check` replaces it.
// - `state.json` holds the last acceptance run and the last check control's results, one entry per check.
// - The controls file is compacted past `CONTROLS_BYTES`, as `controls.ts` says.
//
// # Known gaps
//
// - The disk gate records no reason, so the loop shows only `waiting`.
// - A stop left unread by a worker that died stops the next run at once, unless a `start` follows it.

import { existsSync } from "node:fs";
import { join } from "node:path";
import { goalPath, Refused } from "../goal/model.ts";
import { freeBytes, GIB, type Mutex, openMutex } from "../os/index.ts";
import { readState, writeState } from "../store/state.ts";
import type { Report } from "../verify/accept.ts";
import type { AgentAdapter } from "./adapter.ts";
import { appendControl, type Control, ControlReader, canLift, compact, pauseOwner, type Read, standingPause } from "./controls.ts";
import { loadConfig } from "../goal/config.ts";
import { addNote, Driver, LANDING_PROMPT, type Outcome, type Policy, parseSettings, REPAIR_PROMPT, readPrompts, SESSION_PROMPT, type Settings, setupHold } from "./driver.ts";
import type { Side } from "./catch-up.ts";
import type { HookEvent } from "./hooks.ts";
import type { Lander, Step, Waits } from "./landing.ts";
import { Controls, type Live } from "./session.ts";
import { Walls } from "./wall.ts";

/** The worker's state file in the loop's folder. */
export const STATE_FILE = "state.json";
/** How often the worker reads new controls, in ms. */
export const POLL_MS = 250;
/** How long the disk gate waits before it reads the free space again, in ms. */
export const DISK_RECHECK = 60_000;

/** A control the run has yet to act on: its action, when it arrived, and who asked. */
export interface Pending {
  action: string;
  at: number;
  by: string;
}

/** One check's result, as a `check` control left it. */
export interface CheckRan {
  id: string;
  green: boolean;
}

/** What `state.json` holds. */
export interface WorkerState {
  pid: number;
  /** When the run started, in ms since the Unix epoch. */
  started: number;
  state: "running" | "checking" | "waiting" | "paused" | "landing" | "idle";
  ask: Pending | null;
  /** Who owns the pause the loop holds under, or null. */
  pausedBy: string | null;
  /** How many turns the run has ended. */
  turns: number;
  /** The byte offset in `controls.ndjson` the worker has read to. */
  controlsRead: number;
  /** The acceptance run as it stands, until the next session starts. */
  checking: Report[] | null;
  /** The results of the last `check` control, with why it could not run, or empty. */
  checked: { checks: CheckRan[]; error: string } | null;
  /** Why the run ended, once it has. */
  ended: string | null;
}

/** The worker found the loop's mutex held: another worker runs the loop. */
export class Taken extends Error {}

/** Everything a worker starts with. Each field it leaves out takes its default. */
export interface WorkerOptions {
  /** The loop's worktree, the project root of a main loop. */
  root: string;
  /** The loop's folder, `.looper/loops/<name>/`. */
  dir: string;
  adapter: AgentAdapter;
  /** The usage walls, `.looper/walls.json` of the root by default. */
  walls?: Walls;
  /** The account the agent runs on, `[agent] adapter` by default. */
  account?: string;
  loopName?: string;
  policy?: Policy;
  live?: (e: Live) => void;
  /** A side loop's goal and landing places, and its landing. Both are left out for a main loop. */
  side?: Side;
  lander?: Lander;
}

/** What the controls have asked of the run between sessions. */
interface Asks {
  stop: Pending | null;
  paused: Pending | null;
  /** Counts `retry-now`, so a wait sees one that arrives while it waits. */
  retry: number;
  /** The `set` controls not yet laid over the sessions ahead, merged key by key. */
  set: Settings | null;
  skipStage: boolean;
  /** A `check` not yet run, with the name it narrows to, or empty for all. */
  check: string | null;
}

function taken(action: string, by: string): Pending {
  return { action, at: Date.now(), by };
}

/** The name of the mutex a worker of the loop whose folder is `dir` holds. */
function loopMutex(dir: string): string {
  const key = Bun.hash(dir.replaceAll("/", "\\").replace(/\\+$/, "").toLowerCase()).toString(16);
  return `Local\\looper-kit-loop-${key}`;
}

/** Whether a worker of another process runs the loop whose folder is `dir`: it holds the loop's mutex. */
export function workerRuns(dir: string): boolean {
  const m = openMutex(loopMutex(dir));
  const free = m.tryTake();
  m.close();
  return !free;
}

/**
 * Ends the landing `l` holds at `step`, past the fast-forward, with no driver, because the worktree may
 * be gone. It holds the loop's mutex meanwhile, as a worker does, and writes `idle` with why it ended.
 * Throws `Taken` when another worker runs the loop.
 */
export async function finishLanding(dir: string, l: Lander, step: Step): Promise<string> {
  const mutex: Mutex = openMutex(loopMutex(dir));
  if (!mutex.tryTake()) {
    mutex.close();
    throw new Taken(`another worker runs the loop in ${dir}`);
  }
  try {
    const landed = await l.finish(step);
    const why = landed.kind === "done" || landed.kind === "hold" ? landed.why : landed.kind;
    const path = join(dir, STATE_FILE);
    const last = readState<WorkerState>(path);
    writeState(path, { ...last, pid: process.pid, state: "idle", ask: null, pausedBy: null, ended: why });
    return why;
  } finally {
    mutex.close();
  }
}

/** The controls a starting worker acts on: those past `from` and after the last `start` among them. */
function unread(all: Read[], from: number): Read[] {
  const fresh = all.filter((r) => r.end > from);
  const last = fresh.findLastIndex((r) => r.control.action === "start");
  return fresh.slice(last + 1);
}

/** One main loop's run. */
export class Worker {
  readonly driver: Driver;
  readonly dir: string;
  private asks: Asks = { stop: null, paused: null, retry: 0, set: null, skipStage: false, check: null };
  private readonly session = new Controls();
  private readonly reader: ControlReader;
  private readonly statePath: string;
  private st: WorkerState;
  /** Whether the `before_run` hooks have passed in this run. */
  private started = false;
  private readonly lander: Lander | undefined;
  /** Whether the loop held a landing when the worker started, which the first turn goes on with. */
  private landingDue: boolean;
  /** The catch-ups the landing has run while the others were paused, since the run's last hold. */
  private readonly rounds = { n: 0 };

  constructor(o: WorkerOptions) {
    this.dir = o.dir;
    this.statePath = join(o.dir, STATE_FILE);
    this.reader = new ControlReader(o.dir);
    const status = join(o.dir, "status.txt");
    const cfg = loadConfig(o.root);
    this.driver = new Driver({
      root: o.root,
      dir: o.dir,
      cfg,
      adapter: o.adapter,
      walls: o.walls ?? new Walls(join(o.root, ".looper", "walls.json")),
      account: o.account ?? cfg.agent.adapter,
      statusFile: status,
      checkLogs: join(o.dir, "checks"),
      hookLogs: join(o.dir, "hooks"),
      loopName: o.loopName,
      loopKind: o.side === undefined ? "main" : "side",
      side: o.side,
      // The prompts are read before each turn, where one that cannot be read holds the loop.
      prompts: { session: SESSION_PROMPT, repair: REPAIR_PROMPT, landing: LANDING_PROMPT },
      policy: o.policy,
      live: o.live,
      progress: (run) => this.checking(run),
    });
    this.st = {
      pid: process.pid,
      started: Date.now(),
      state: "running",
      ask: null,
      pausedBy: null,
      turns: 0,
      controlsRead: 0,
      checking: null,
      checked: null,
      ended: null,
    };
    this.lander = o.lander;
    this.landingDue = o.lander?.held() != null;
    // A side goal retired before the worker started goes on with its catch-up.
    const side = o.side;
    if (!this.landingDue && side !== undefined && !existsSync(goalPath(join(o.root, cfg.project.goals_dir), side.goal))) {
      this.driver.state.catchingUp = side.goal;
    }
  }

  /** Runs the loop until its run ends, and returns why it ended. Throws `Taken` when another worker runs it. */
  async run(): Promise<string> {
    const mutex: Mutex = openMutex(loopMutex(this.dir));
    if (!mutex.tryTake()) {
      mutex.close();
      throw new Taken(`another worker runs the loop in ${this.dir}`);
    }
    let timer: Timer | undefined;
    let why: string;
    try {
      this.begin();
      timer = setInterval(() => this.poll(), POLL_MS);
      why = await this.turns();
    } catch (e) {
      why = `error: ${(e as Error).message}`;
      throw e;
    } finally {
      clearInterval(timer);
      this.poll();
      this.st.ended = why!;
      this.name("idle");
      mutex.close();
    }
    return why;
  }

  /** Folds the controls file for the pause that stands, and acts on the controls this run is due. */
  private begin(): void {
    const last = readState<WorkerState>(this.statePath);
    const all: Read[] = [];
    for (let page = this.reader.read(); page.length > 0; page = this.reader.read()) all.push(...page);
    const pause = standingPause(all.map((r) => r.control));
    if (pause !== null) this.asks.paused = taken("pause", pause.by);
    const from = last !== undefined && last.controlsRead <= this.reader.offset ? last.controlsRead : 0;
    for (const r of unread(all, from)) this.apply(r.control);
    this.st.controlsRead = this.reader.offset;
    this.name(this.asks.paused !== null ? "paused" : "running");
  }

  /** Reads the controls appended since the last read and acts on each. */
  private poll(): void {
    const read = this.reader.read();
    for (const r of read) this.apply(r.control);
    if (read.length > 0) {
      this.st.controlsRead = this.reader.offset;
      this.write();
    }
  }

  /** Acts on one control: sets what it asks of the run, or hands it to the running session. */
  private apply(c: Control): void {
    if (c.refused !== undefined) return;
    const a = this.asks;
    switch (c.action) {
      case "stop":
        a.stop ??= taken("stop", c.by);
        break;
      case "stop-now":
        a.stop = taken("stop-now", c.by);
        this.session.send({ kind: "kill", why: "the operator stopped the loop now" });
        break;
      case "pause":
        if (a.paused !== null) a.paused.by = pauseOwner(a.paused.by, c.by);
        else a.paused = taken("pause", c.by);
        break;
      case "resume": {
        const owner = a.paused?.by ?? "";
        if (!canLift(c.by, owner)) {
          this.record("warn", `${c.by} cannot lift the pause ${owner} made, so the loop stays paused`).catch(report);
          break;
        }
        a.paused = null;
        break;
      }
      case "retry-now":
        a.retry++;
        break;
      case "skip-stage":
        a.skipStage = true;
        break;
      case "set":
        // `loop set` refuses a payload `parseSettings` refuses, so one that fails here is dropped.
        try {
          a.set = { ...a.set, ...parseSettings(c.payload) };
        } catch {}
        break;
      case "check":
        a.check = c.payload;
        break;
      case "say":
        this.session.send({ kind: "say", text: c.payload });
        break;
      case "interrupt":
        this.session.send({ kind: "interrupt", text: c.payload });
        break;
      case "freeze":
        this.session.send({ kind: "freeze" });
        break;
      case "thaw":
        this.session.send({ kind: "thaw" });
        break;
    }
  }

  /** Appends a control by `looper` to the loop's controls. */
  private async record(action: string, payload: string): Promise<void> {
    await appendControl(this.dir, { by: "looper", action, payload });
  }

  /** Writes `state.json` as the run stands. */
  private write(): void {
    const a = this.asks;
    const s = this.st;
    s.ask = a.stop ?? (s.state !== "paused" ? a.paused : null);
    s.pausedBy = s.state === "paused" ? (a.paused?.by ?? null) : null;
    writeState(this.statePath, s);
  }

  /** Names `state` in `state.json`. */
  private name(state: WorkerState["state"]): void {
    this.st.state = state;
    this.write();
  }

  /** Names `checking` and keeps `run`: the driver's progress sink. */
  private checking(run: Report[]): void {
    this.st.checking = run;
    this.name("checking");
  }

  /** Waits until `done` holds of the asks, or until `deadline`. True when `done` held. */
  private async until(done: (a: Asks) => boolean, deadline?: number): Promise<boolean> {
    for (;;) {
      if (done(this.asks)) return true;
      const left = deadline === undefined ? POLL_MS : deadline - Date.now();
      if (left <= 0) return false;
      await Bun.sleep(Math.min(left, POLL_MS));
    }
  }

  /** Runs turns until one ends the run or a stop is asked. Returns why the run ended. */
  private async turns(): Promise<string> {
    const d = this.driver;
    for (;;) {
      const rest = await compact(this.reader);
      if (rest !== null) {
        for (const r of rest) this.apply(r.control);
        this.st.controlsRead = this.reader.offset;
        this.write();
      }
      if (!(await this.reloaded())) {
        if (this.asks.stop !== null) return "stopped";
        continue;
      }
      await this.between();
      await this.warned();
      if (this.asks.stop !== null) return "stopped";
      if (this.asks.paused !== null) {
        this.name("paused");
        await this.until((a) => a.stop !== null || a.paused === null || a.check !== null);
        continue;
      }
      if (this.lowDisk()) {
        this.name("waiting");
        await this.until((a) => a.stop !== null || a.paused !== null || a.check !== null, Date.now() + DISK_RECHECK);
        continue;
      }
      const why = await this.ready();
      if (why !== null) {
        await this.hold(setupHold(why));
        continue;
      }
      // Controls left from between sessions: a say or an interrupt waits for the next session, the rest lapse.
      for (let c = this.session.take(); c !== undefined; c = this.session.take()) {
        if (c.kind === "say" || c.kind === "interrupt") addNote(d.state, c.text);
      }
      this.st.checking = null;
      this.name("running");
      const outcome = await this.turn();
      this.st.turns++;
      this.write();
      switch (outcome.kind) {
        case "advanced":
        case "continue":
          this.name("running");
          break;
        case "wait":
          await this.wait(outcome.until);
          break;
        case "hold":
          await this.hold(outcome.why);
          break;
        case "end":
          return outcome.why;
        case "land": {
          const why = await this.land(outcome.slug);
          if (why !== null) return why;
          break;
        }
      }
    }
  }

  /** Lands side goal `slug`'s branch (`Lander.land`). Returns why the run ends, or null when it goes on: with a landing session, or held, from a fresh round count, to catch up again on a resume. */
  private async land(slug: string): Promise<string | null> {
    const l = this.lander;
    if (l === undefined) return `${slug} is retired, but only a side loop lands`;
    this.name("landing");
    const waits: Waits = {
      until: (done, deadline) => this.until(done, deadline),
      stopped: () => this.asks.stop !== null,
      retries: () => this.asks.retry,
    };
    const landed = await l.land(this.driver, waits, this.rounds);
    let held: string;
    switch (landed.kind) {
      case "done":
        return landed.why;
      case "stopped":
        return "stopped";
      case "hold":
        held = landed.why;
        break;
      case "again":
        if (landed.outcome.kind === "end") return landed.outcome.why;
        if (landed.outcome.kind !== "hold") {
          this.name("running");
          return null;
        }
        held = landed.outcome.why;
        break;
    }
    this.rounds.n = 0;
    this.driver.state.catchingUp = slug;
    await this.hold(held);
    return null;
  }

  /** Waits for a turn's `wait` until `at`. A pause or a stop ends it sooner; a `retry-now` ends it and lifts the account's usage wall; a check runs and the wait goes on. */
  private async wait(at: number): Promise<void> {
    this.name("waiting");
    const retry = this.asks.retry;
    for (;;) {
      const woke = await this.until((a) => a.stop !== null || a.retry !== retry || a.check !== null || a.paused !== null, at);
      const a = this.asks;
      const [stop, retried, paused] = [a.stop !== null, a.retry !== retry, a.paused !== null];
      if (woke && !stop && !retried && !paused) {
        await this.between();
        continue;
      }
      if (woke && !stop && retried) await this.driver.walls.liftShared(this.driver.account);
      return;
    }
  }

  /** Runs one turn (`Driver.turn`), records its hook warnings, and runs the `after_session` hooks after a turn that started a session. A worker that started mid-landing goes on with the landing instead, once. */
  private async turn(): Promise<Outcome> {
    const d = this.driver;
    if (this.lander !== undefined && this.landingDue) {
      this.landingDue = false;
      return { kind: "land", slug: this.lander.slug, said: "" };
    }
    const sessions = d.state.sessions;
    let outcome = await d.turn(this.session);
    await this.warned();
    if (d.state.sessions !== sessions) {
      const paused = await this.hook("after_session");
      d.state.session = null;
      if (paused !== null && outcome.kind !== "hold" && outcome.kind !== "end") {
        // A branch held before it lands catches up again on the resume.
        if (outcome.kind === "land") d.state.catchingUp = outcome.slug;
        outcome = { kind: "hold", why: setupHold(paused) };
      }
    }
    return outcome;
  }

  /** Reads `looper.toml` again. One that does not load holds the loop with its error until a resume or a stop, and returns false. */
  private async reloaded(): Promise<boolean> {
    try {
      this.driver.reload();
      return true;
    } catch (e) {
      if (this.asks.stop !== null) return false;
      if (this.asks.paused === null) await this.hold(setupHold(`looper.toml does not load: ${(e as Error).message}`));
      this.name("paused");
      await this.until((a) => a.stop !== null || a.paused === null);
      return false;
    }
  }

  /** Hands the driver what the controls asked between sessions: the settings and the stage skip, and a check, which runs now under `checking`. */
  private async between(): Promise<void> {
    const a = this.asks;
    const d = this.driver;
    if (a.set !== null) d.set(a.set);
    a.set = null;
    d.state.skipStage ||= a.skipStage;
    a.skipStage = false;
    const text = a.check;
    a.check = null;
    if (text === null) return;
    const before = this.st.state;
    try {
      const verdicts = await d.check(text);
      this.st.checked = { checks: verdicts.map((v) => ({ id: v.id, green: v.green })), error: "" };
    } catch (e) {
      if (!(e instanceof Refused)) console.error(`error: the check control: ${(e as Error).message}`);
      this.st.checked = { checks: [], error: (e as Error).message };
    }
    this.name(before);
  }

  /** Readies the next session: reads its prompts, runs the `before_run` hooks until they pass once, and the `before_session` hooks. Returns why the loop holds instead. */
  private async ready(): Promise<string | null> {
    const d = this.driver;
    try {
      d.prompts = readPrompts(d.root, d.cfg.prompts, d.statusFile);
    } catch (e) {
      return (e as Error).message;
    }
    if (!this.started) {
      const why = await this.hook("before_run");
      if (why !== null) return why;
      this.started = true;
    }
    return this.hook("before_session");
  }

  /** True when the worktree's volume has less free space than `[loop] min_free_gb`. */
  private lowDisk(): boolean {
    const min = this.driver.cfg.loop.min_free_gb;
    if (min === 0) return false;
    let free: number;
    try {
      free = freeBytes(this.driver.root);
    } catch (e) {
      throw new Error(`cannot read the free space of ${this.driver.root}: ${(e as Error).message}`);
    }
    return free < min * GIB;
  }

  /** Runs `event`'s hooks, records their warnings, and returns why the hook that pauses the loop failed. */
  private async hook(event: HookEvent): Promise<string | null> {
    const paused = await this.driver.hooks(event);
    await this.warned();
    return paused;
  }

  /** Records each hook warning the driver holds as a `warn` control by `looper`. */
  private async warned(): Promise<void> {
    const s = this.driver.state;
    const warnings = s.warnings;
    s.warnings = [];
    for (const w of warnings) await this.record("warn", w);
  }

  /** Pauses the loop, and records the pause with `why` as a control by `looper`. */
  private async hold(why: string): Promise<void> {
    await this.record("pause", why);
    this.asks.paused ??= taken("pause", "looper");
  }
}

function report(e: unknown): void {
  console.error(`error: ${(e as Error).message}`);
}
