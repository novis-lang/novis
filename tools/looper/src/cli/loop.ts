// `loop`, `status` and `log`: a project's loops, run, controlled and read from their folders in its data
// folder, as `crates/looper/src/cmd/live.rs` does through looper's API. A loop is
// `.looper/loops/<name>/` in the project of the current directory (decision 0027). The worker
// (`../loop/worker.ts`) is the home of what a loop does with each control.
//
// - `loop run` is the loop's worker in the foreground, until its run ends. It exits 4 when another
//   worker runs the loop. It drops `LOOPER_ASKER`, so a session it runs asks as `agent`. The hidden `--stand-in <script>` replaces the agent with the stand-in.
// - `loop add` adds the project's main loop, idle. `loop add --side <slug> [--from <ref>]` adds the side
//   loop `side-<slug>` in a lane, or waiting for one. `../loop/lanes.ts` is the home of both. A loop
//   that exists already is refused.
// - `loop run` works in the loop's worktree: the root for a main loop, its lane or own worktree for a
//   side loop. A side loop that waits for a lane refuses `run` and `start`.
// - A side loop's worker lands its branch on the main loop's branch, or the root's when there is none,
//   after a catch-up that runs the kit's own `verify` (`../loop/landing.ts`). The loop it hands its lane
//   to starts with the same agent. A landed loop leaves `status` and refuses `run`. A worker that starts
//   past its landing's fast-forward only finishes it.
// - `loop start` of a loop whose worker runs reports the loop and exits 0, and appends nothing.
//   Otherwise it appends a `start` and starts `loop run` detached through the OS module.
// - Every other `loop` verb appends one control by the same name with `appendControl`, as its asker
//   (`asker` in `claims.ts`). A loop with no worker running refuses it, as looper's API does. A resume
//   that cannot lift the standing pause is refused with the reason. `loop set` refuses a payload the
//   worker cannot use.
// - `status` lists every loop of the project with its `loop.json` and `state.json`. A loop whose worker
//   does not run is `idle`, and one whose worker died says so in `ended`. A side loop that waits for a
//   lane is `waiting`, and its `waiting` names why.
// - `log <session>` prints a session's events, a page at a time from `--from`, so its memory is one page
//   whatever the session's length. `--follow` reads again every `FOLLOW_EVERY` until the session has
//   ended and its last page is printed. `--search <text>` gives the events whose summary holds the
//   text instead (`../loop/log.ts`).
//
// Every command takes `--loop <name>`, `main` by default. With `--json`, `loop run` prints `{"loop",
// "ended"}`, `add` and a `start` that sent nothing the loop's `status` entry, and a control `{"loop",
// "action", "control", "pid"}`. `status` prints `{"loops"}`. `log` prints one page, `{"events", "next",
// "ended"}`, and with `--search` `{"hits", "next"}`. `--follow` takes no `--json`, since one document
// cannot grow with the session.

import { existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { loadConfig } from "../goal/config.ts";
import { Refused } from "../goal/model.ts";
import { looperHome } from "../jobs/slots.ts";
import type { AgentAdapter } from "../loop/adapter.ts";
import { Claude } from "../loop/claude.ts";
import { appendControl } from "../loop/controls.ts";
import { parseSettings } from "../loop/driver.ts";
import type { Side } from "../loop/catch-up.ts";
import { Lander, pastLand, type Step } from "../loop/landing.ts";
import { addLoop, type LoopRecord, laneWaiting, loopRecord, loops, loopsDir, type Waiting } from "../loop/lanes.ts";
import { events, PAGE_EVENTS, search, sessionDir, sessionRecord } from "../loop/log.ts";
import { StandIn } from "../loop/stand-in.ts";
import { Walls } from "../loop/wall.ts";
import { finishLanding, STATE_FILE, Taken, Worker, type WorkerState, workerRuns } from "../loop/worker.ts";
import { spawnDetached } from "../os/index.ts";
import { readState } from "../store/state.ts";
import { asker } from "./claims.ts";
import { type Command, type Context, Exit, Refusal, type Report, usage } from "./cli.ts";

/** How often `log --follow` reads the session again, in ms. */
export const FOLLOW_EVERY = 1_000;

const MAIN = join(import.meta.dir, "..", "..", "main.ts");

/** The verbs that are one control each, with whether each takes a text. */
const CONTROLS: Record<string, boolean> = {
  stop: false,
  "stop-now": false,
  pause: false,
  resume: false,
  freeze: false,
  thaw: false,
  say: true,
  interrupt: true,
  "retry-now": false,
  "skip-stage": false,
};

/** The `loop set` flags, each with its key in the `set` payload and whether its value is a number. */
const SETTINGS: Record<string, [string, boolean]> = {
  "--model": ["model", false],
  "--effort": ["effort", false],
  "--budget-hard": ["budget.hard", true],
  "--budget-kill": ["budget.kill", true],
  "--max-sessions": ["max_sessions", true],
};

/** The words of `args` with `--loop <name>` taken out, and the name, `main` by default. */
export function loopName(args: string[]): { name: string; named: boolean; rest: string[] } {
  const rest: string[] = [];
  let name = "main";
  let named = false;
  for (let i = 0; i < args.length; i++) {
    const a = args[i] as string;
    if (a === "--loop" || a.startsWith("--loop=")) {
      const value = a === "--loop" ? args[++i] : a.slice("--loop=".length);
      if (value === undefined || !/^[A-Za-z0-9][A-Za-z0-9_-]*$/.test(value)) throw usage("`--loop` takes a loop's name: letters, digits, `-` and `_`");
      name = value;
      named = true;
    } else rest.push(a);
  }
  return { name, named, rest };
}

/** The value of option `flag` in `args`, which it takes out, as `--flag <value>` or `--flag=<value>`. */
export function option(args: string[], flag: string): string | undefined {
  for (let i = 0; i < args.length; i++) {
    const a = args[i] as string;
    if (a === flag) {
      const value = args[i + 1];
      if (value === undefined) throw usage(`\`${flag}\` takes a value`);
      args.splice(i, 2);
      return value;
    }
    if (a.startsWith(`${flag}=`)) {
      args.splice(i, 1);
      return a.slice(flag.length + 1);
    }
  }
  return undefined;
}

/** Whether `args` holds `flag`, which it takes out. */
export function flag(args: string[], name: string): boolean {
  const at = args.indexOf(name);
  if (at < 0) return false;
  args.splice(at, 1);
  return true;
}

/** A whole number of at least `min` from option `name`'s `value`. */
export function whole(name: string, value: string, min: number): number {
  if (!/^\d+$/.test(value) || !Number.isSafeInteger(Number(value)) || Number(value) < min) throw usage(`\`${name}\` takes a whole number of at least ${min}`);
  return Number(value);
}

export function noMore(args: string[], what: string): void {
  if (args.length > 0) throw usage(`\`${what}\` does not take \`${args[0]}\``);
}

/** A loop as `status` lists it: its `loop.json` and `state.json`, with `idle` when its worker does not run. */
export interface LoopView extends Partial<Omit<WorkerState, "pid">>, LoopRecord {
  name: string;
  state: WorkerState["state"];
  pid: number | null;
  running: boolean;
  ended: string | null;
  /** Why a side loop waits for a lane, and since when, or null. */
  waiting: Waiting | null;
}

/** The number of lanes the project at `root` has, 2 when its `looper.toml` does not load. */
function lanesOf(root: string): number {
  try {
    return loadConfig(root).loop.lanes;
  } catch {
    return 2;
  }
}

/** The loop whose folder is `dir`, named `name`, of the project at `root`, as `status` lists it. */
function view(root: string, name: string, dir: string): LoopView {
  const record = loopRecord(root, dir);
  const waiting = laneWaiting(record, lanesOf(root));
  const st = readState<WorkerState>(join(dir, STATE_FILE));
  const running = workerRuns(dir);
  if (running && st !== undefined) return { ...st, ...record, name, running, waiting };
  const ended = st === undefined ? null : st.state === "idle" ? st.ended : (st.ended ?? "the worker died");
  const state = waiting === null ? "idle" : "waiting";
  return { ...st, ...record, name, state, pid: running ? (st?.pid ?? null) : null, running, ended, waiting };
}

function line(v: LoopView): string {
  const worker = v.running ? `worker ${v.pid ?? "starting"}` : "no worker";
  const ended = !v.running && v.ended !== null ? `, ended: ${v.ended}` : "";
  const paused = v.state === "paused" && v.pausedBy ? ` by ${v.pausedBy}` : "";
  const why = v.waiting === null ? "" : `: ${v.waiting.why}`;
  const lane = v.lane === null ? "" : ` in lane-${v.lane}`;
  return `${v.name}  ${v.state}${paused}${why}${lane}  (${worker}${ended})`;
}

function status(root: string): Report {
  const all = loops(root)
    .filter((l) => l.landed === undefined)
    .map((l) => view(root, l.name, join(loopsDir(root), l.name)));
  const text = all.length === 0 ? "this project has no loops yet: `loop add` adds its main loop" : all.map(line).join("\n");
  return { doc: { loops: all }, text };
}

/** The loop's folder, or a refusal when the project has no such loop. */
function existing(root: string, name: string): string {
  const dir = join(loopsDir(root), name);
  if (!existsSync(dir)) throw new Refusal(`this project has no loop ${name}: \`loop add\` adds its main loop`);
  return dir;
}

async function add(root: string, name: string | undefined, args: string[]): Promise<Report> {
  const side = option(args, "--side");
  const from = option(args, "--from");
  noMore(args, "loop add");
  let added: Awaited<ReturnType<typeof addLoop>>;
  try {
    added = await addLoop(root, { name, side, from });
  } catch (e) {
    throw new Refusal((e as Error).message);
  }
  const v = view(root, added.name, join(loopsDir(root), added.name));
  const where = v.waiting !== null ? `, waiting: ${v.waiting.why}` : ` in ${v.worktree}`;
  return { doc: v, text: `loop ${added.name} added on ${added.branch}${where}` };
}

/** The worktree loop `name`'s worker works in, or a refusal while the loop waits for a lane. */
function worktreeOf(root: string, name: string, dir: string): string {
  const record = loopRecord(root, dir);
  const waiting = laneWaiting(record, lanesOf(root));
  if (waiting !== null) throw new Refusal(`loop ${name} cannot start: it waits, ${waiting.why}`);
  return record.worktree;
}

/** The agent `loop run` gives its worker: the stand-in with a script, or `[agent] adapter`. */
function adapter(root: string, script: string | undefined): AgentAdapter {
  if (script !== undefined) return new StandIn(script);
  let name: string;
  try {
    name = loadConfig(root).agent.adapter;
  } catch (e) {
    throw new Refusal((e as Error).message);
  }
  if (name !== "claude") throw new Refusal(`the kit has no agent adapter "${name}"`);
  return new Claude();
}

/** What side loop `record` of the project at `root` lands with: its goal, the main loop's branch or the root's, and the kit's own verify. */
function sideOf(root: string, record: LoopRecord): Side {
  const main = loops(root).find((l) => l.kind === "main")?.branch;
  const head = () => Bun.spawnSync(["git", "symbolic-ref", "-q", "--short", "HEAD"], { cwd: root, stdin: "ignore", stdout: "pipe", stderr: "ignore" }).stdout.toString().trim();
  return { goal: record.goal ?? "", mainRoot: root, mainBranch: main ?? head(), verify: [process.execPath, MAIN, "verify"] };
}

async function runLoop(root: string, name: string, args: string[]): Promise<Report> {
  const script = option(args, "--stand-in");
  noMore(args, "loop run");
  // The asker of the `start` is on record. A session, a check or a hook the worker runs asks as itself.
  delete process.env.LOOPER_ASKER;
  const dir = join(loopsDir(root), name);
  mkdirSync(dir, { recursive: true });
  const record = loopRecord(root, dir);
  if (record.landed !== undefined) throw new Refusal(`loop ${name} has landed`);
  const side = record.kind === "side" ? sideOf(root, record) : undefined;
  // The side loop the landing hands its lane to starts with this worker's agent.
  const lander = side && new Lander({ name, worktree: record.worktree, side, start: (next) => void startWorker(root, next, script, "looper").catch(report) });
  try {
    const step = lander?.held() ?? null;
    let ended: string;
    if (lander !== undefined && pastLand(step)) {
      ended = await finishLanding(dir, lander, step as Step);
    } else {
      const worktree = worktreeOf(root, name, dir);
      let worker: Worker;
      try {
        // A stand-in's walls are shared only with the service of a home `LOOPER_HOME` names, never the installed one.
        const shared = script === undefined || (process.env.LOOPER_HOME ?? "") !== "" ? looperHome() : null;
        const walls = new Walls(join(root, ".looper", "walls.json"), shared);
        worker = new Worker({ root: worktree, dir, adapter: adapter(root, script), walls, loopName: name, side, lander });
      } catch (e) {
        throw e instanceof Refusal ? e : new Refusal((e as Error).message);
      }
      ended = await worker.run();
    }
    return { doc: { loop: name, ended }, text: `loop ${name} ended: ${ended}` };
  } catch (e) {
    if (e instanceof Taken) throw new Refusal(e.message, Exit.taken);
    throw e;
  }
}

/** Appends a `start` by `by` to loop `name` and starts its `loop run` detached, with the stand-in `script` when one is named. */
async function startWorker(root: string, name: string, script: string | undefined, by: string) {
  const control = await appendControl(join(loopsDir(root), name), { by, action: "start" });
  const argv = [process.execPath, MAIN, "loop", "run", `--loop=${name}`];
  // One argument with `=`, so a script path is never read as a flag.
  if (script !== undefined) argv.push(`--stand-in=${script}`);
  return { control, child: spawnDetached(argv, { cwd: root }) };
}

function report(e: unknown): void {
  console.error(`error: ${(e as Error).message}`);
}

async function start(root: string, name: string, args: string[]): Promise<Report> {
  const script = option(args, "--stand-in");
  noMore(args, "loop start");
  const dir = existing(root, name);
  if (workerRuns(dir)) {
    const v = view(root, name, dir);
    return { doc: v, text: `loop ${name} runs (worker ${v.pid ?? "starting"})` };
  }
  worktreeOf(root, name, dir);
  const { control, child } = await startWorker(root, name, script, asker());
  const bound = child.bound ? "; it runs in this job, which lets no process leave, and ends with it" : "";
  return { doc: { loop: name, action: "start", control, pid: child.pid }, text: `loop ${name}: start sent, worker ${child.pid}${bound}` };
}

/** The action and payload of control verb `verb` with its words `args`. */
function parts(verb: string, args: string[]): [string, string] {
  if (verb === "check") {
    const only = option(args, "--name");
    noMore(args, "loop check");
    return ["check", only ?? ""];
  }
  if (verb === "set") {
    const set: Record<string, string | number> = {};
    for (const [f, [key, number]] of Object.entries(SETTINGS)) {
      const value = option(args, f);
      if (value !== undefined) set[key] = number ? whole(f, value, 1) : value;
    }
    noMore(args, "loop set");
    if (Object.keys(set).length === 0) throw usage(`\`loop set\` takes at least one of ${Object.keys(SETTINGS).join(", ")}`);
    const payload = JSON.stringify(set);
    try {
      parseSettings(payload);
    } catch (e) {
      throw new Refusal((e as Error).message);
    }
    return ["set", payload];
  }
  const text = CONTROLS[verb];
  if (text === undefined) throw usage(`\`loop\` takes add, run, start, check, set or a control, and got \`${verb}\``);
  if (!text) {
    noMore(args, `loop ${verb}`);
    return [verb, ""];
  }
  if (args.length !== 1) throw usage(`\`loop ${verb}\` takes one text`);
  return [verb, args[0] as string];
}

async function control(root: string, name: string, verb: string, args: string[]): Promise<Report> {
  const [action, payload] = parts(verb, args);
  const dir = existing(root, name);
  if (!workerRuns(dir)) throw new Refusal(`loop ${name} has no worker running: \`loop start\` starts one`);
  let c: Awaited<ReturnType<typeof appendControl>>;
  try {
    c = await appendControl(dir, { by: asker(), action, payload });
  } catch (e) {
    throw e instanceof Refused ? new Refusal(e.message) : e;
  }
  const pid = readState<WorkerState>(join(dir, STATE_FILE))?.pid ?? null;
  return { doc: { loop: name, action, control: c, pid }, text: `loop ${name}: ${action} sent${pid === null ? "" : `, worker ${pid}`}` };
}

export const loopCommand: Command = {
  name: "loop",
  about: "Runs, starts and controls this project's loops: `loop add|run|start|stop|pause|resume|say|…`.",
  json: true,
  run: (argv) => {
    const { name, named, rest } = loopName(argv);
    const [verb, ...args] = rest;
    const root = process.cwd();
    switch (verb) {
      case undefined:
        throw usage("`loop` takes add, run, start, check, set or a control");
      case "add":
        return add(root, named ? name : undefined, args);
      case "run":
        return runLoop(root, name, args);
      case "start":
        return start(root, name, args);
      default:
        return control(root, name, verb, args);
    }
  },
};

export const statusCommand: Command = {
  name: "status",
  about: "Lists this project's loops, each with its state and its worker.",
  json: true,
  run: (args) => {
    noMore(args, "status");
    return status(process.cwd());
  },
};

function eventLine(e: { seq: number; kind: string; summary: string }): string {
  return `${String(e.seq).padStart(6)}  ${e.kind}  ${e.summary}`;
}

async function log(argv: string[], ctx: Context): Promise<Report> {
  const { name, rest: args } = loopName(argv);
  const fromText = option(args, "--from");
  const limitText = option(args, "--limit");
  const text = option(args, "--search");
  const follow = flag(args, "--follow");
  const [session, ...more] = args;
  noMore(more, "log");
  if (session === undefined) throw usage("`log` takes a session's number");
  const n = whole("log", session, 1);
  let from = fromText === undefined ? 0 : whole("--from", fromText, 0);
  const limit = limitText === undefined ? PAGE_EVENTS : whole("--limit", limitText, 1);
  const dir = existing(process.cwd(), name);
  if (sessionRecord(dir, n) === undefined) throw new Refusal(`loop ${name} has no session ${n}`);
  const at = sessionDir(dir, n);

  if (text !== undefined) {
    if (follow) throw usage("`--search` takes no `--follow`");
    if (text.trim() === "") throw usage("`--search` takes a text to find");
    const page = search(at, text, from, limit);
    const lines = page.hits.map(String);
    if (page.next !== null) lines.push(`more from --from ${page.next}`);
    return { doc: page, text: lines.join("\n") || "no event's summary holds it" };
  }
  if (ctx.json) {
    if (follow) throw usage("`--follow` takes no `--json`: page with `--from` instead");
    const page = events(at, from, limit);
    return { doc: { ...page, ended: sessionRecord(dir, n)?.ended != null } };
  }
  for (;;) {
    // Whether the session had ended is read before its page, so the page holds its last events.
    const ended = !follow || sessionRecord(dir, n)?.ended != null;
    const page = events(at, from, limit);
    for (const e of page.events) ctx.line(eventLine(e));
    const last = page.events.at(-1);
    if (last !== undefined) from = last.seq + 1;
    if (page.next !== null) continue;
    if (ended) return {};
    await Bun.sleep(FOLLOW_EVERY);
  }
}

export const logCommand: Command = {
  name: "log",
  about: "Prints a session's events, or with `--search` those whose summary holds a text: `log <session>`.",
  json: true,
  run: log,
};
