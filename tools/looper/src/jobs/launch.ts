// A launch's own files in its job's folder, and its start and its stop. A launch is a job of kind
// `launch` that a supervisor (`supervise.ts`) runs. It ports `crates/looper-job/src/launch.rs`.
//
// - `launch.json` is the `Launch`: the launcher, the project, the port, the phase and when it was
//   ready. The supervisor writes it at each change, as a state file.
// - `stop` is a `Killed`, written by `askStop`. The supervisor reads it, sends Ctrl+Break to the launch,
//   and kills its tree once the launcher's grace has passed.
// - A launcher's folder, `.looper/launch/<name>/`, holds its copy of the program, the supervisor's log
//   and `start.json`.
//
// `liveLaunches` lists the running launches of a project from its job store alone, so every reader
// works without a server.
//
// - `startLaunch` spawns `launch supervise <name>` with no window and no handle of the caller's, out of
//   the caller's job when that job allows it (`spawnDetached`). It then waits until the supervisor
//   writes `start.json`: the job it made, or a refusal. A refusal of a taken port names the port's
//   owner and the next free port as a `Clash`.
// - `stopLaunch` asks a launch to stop and waits for its exit. A launch that has not ended
//   `STOP_SLACK_MS` past its grace has its job killed.
//
// # Known gaps
//
// - A start inside a job that allows no breakaway, such as a Bash call's, runs its supervisor in that
//   job, so the launch ends with the call. looper's service starts it outside the job once goal
//   `kit-supervisor` serves the start.
//
// # The bounds it keeps
//
// A list holds one entry per job folder, which the store's sweep bounds. A project has at most one live
// launch per declared launcher, which the supervisor's mutex keeps. A start waits at most
// `START_WAIT_MS` for its supervisor.

import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { sameProcess, spawnDetached } from "../os/index.ts";
import { readState, writeState } from "../store/state.ts";
import type { Clash } from "./ports.ts";
import { SLOT_VAR } from "./slots.ts";
import { EVENTS_VAR, type JobExit, type JobRecord, type Killed, PARENT_VAR, type Store } from "./store.ts";

/** The file of a launch's `Launch`, in its job's folder. */
export const LAUNCH = "launch.json";
/** The file a stop request is written to, in the job's folder. */
export const STOP = "stop";
/** The file in a launcher's folder where the supervisor names its job, or why it refused. */
export const START = "start.json";
/** The supervisor's own log in a launcher's folder. */
export const SUPERVISOR_LOG = "supervisor.log";
/** The variable that names the Claude Code session that started a launch. */
export const SESSION_VAR = "CLAUDE_CODE_SESSION_ID";
/** How long a start waits for its supervisor to name its job. */
export const START_WAIT_MS = 30_000;
/** How long past a launcher's `stop_grace` a stop waits before it kills the job. */
export const STOP_SLACK_MS = 10_000;
/** How often a start, a stop and the supervisor look again. */
export const TICK_MS = 100;

/** The kit's entry, which the supervisor runs. */
const MAIN = join(import.meta.dir, "..", "..", "main.ts");

/** Where a running launch stands. */
export type Phase = "building" | "starting" | "ready" | "running" | "stopping";

/** What a launch is: its folder's `launch.json`. */
export interface Launch {
  name: string;
  /** The project's name from its `looper.toml`. */
  project: string;
  /** The project's root. */
  root: string;
  /** The port it was given, or the one its ready line named. */
  port: number | null;
  phase: Phase;
  /** When it became ready, in ms since the Unix epoch. */
  ready: number | null;
  /** Whether its windows are kept off the screen. */
  hidden: boolean;
  /** The loop whose session started it, from `LOOPER_LOOP`. */
  loop: string | null;
}

/** A running launch the store holds. */
export interface Live {
  id: string;
  record: JobRecord;
  launch: Launch;
}

/** Why a supervisor refused to start. */
export interface Refusal {
  why: string;
  /** The port that was taken, when that is why. */
  port?: Clash;
}

/** What the supervisor tells `startLaunch`: its job, or why it refused. */
export type Started = { job: string } | { refused: Refusal };

/** A launcher to start, with the port and args that replace the remembered ones. */
export interface Request {
  name: string;
  port?: number;
  args?: string;
}

/** Why `startLaunch` started nothing: the supervisor refused, or it could not run. */
export class NotStarted extends Error {
  constructor(
    message: string,
    readonly refusal?: Refusal,
  ) {
    super(message);
  }
}

/** The launch of job `id`, when the job is one. */
export function readLaunch(store: Store, id: string): Launch | undefined {
  try {
    return readState<Launch>(join(store.dir(id), LAUNCH));
  } catch {
    return undefined;
  }
}

/** Writes `launch` into the job folder `dir`. */
export function writeLaunch(dir: string, launch: Launch): void {
  writeState(join(dir, LAUNCH), launch);
}

/** Who asked the launch in the job folder `dir` to stop, once someone has. */
export function stopAsked(dir: string): Killed | undefined {
  try {
    return readState<Killed>(join(dir, STOP));
  } catch {
    return undefined;
  }
}

/** Asks launch job `id` to stop, writing that `by` asked. */
export function askStop(store: Store, id: string, by: string): void {
  writeState(join(store.dir(id), STOP), { by, at: Date.now() } satisfies Killed);
}

/** The running launches of the project whose store is `store`, newest first. */
export function liveLaunches(store: Store): Live[] {
  const out: Live[] = [];
  for (const e of store.list()) {
    if (e.record.kind !== "launch" || e.state.state !== "running") continue;
    const launch = readLaunch(store, e.id);
    if (launch) out.push({ id: e.id, record: e.record, launch });
  }
  return out;
}

/** The folder of launcher `name` in the project at `root`. */
export function launchDir(root: string, name: string): string {
  return join(root, ".looper", "launch", name);
}

/** Writes what the supervisor tells `startLaunch` into the launcher's folder `dir`. */
export function tell(dir: string, said: Started): void {
  writeState(join(dir, START), said);
}

/** This process's environment for a supervisor: a job of its own, under no other job and holding no slot. */
function supervisorEnv(): Record<string, string | undefined> {
  const dropped = [PARENT_VAR, SLOT_VAR, EVENTS_VAR].map((k) => k.toLowerCase());
  return Object.fromEntries(Object.entries(process.env).filter(([k]) => !dropped.includes(k.toLowerCase())));
}

/** Starts launcher `req.name` of the project at `root`, as the head comment says. Gives the launch's job, and whether it ends with this process's job. */
export async function startLaunch(root: string, req: Request): Promise<{ id: string; bound: boolean }> {
  const dir = launchDir(root, req.name);
  const started = join(dir, START);
  mkdirSync(dir, { recursive: true });
  rmSync(started, { force: true });
  writeFileSync(join(dir, SUPERVISOR_LOG), "");
  const argv = [process.execPath, MAIN, "launch", "supervise", req.name];
  if (req.port !== undefined) argv.push("--port", String(req.port));
  // One argument with `=`, so an args text that starts with `-` is never read as a flag.
  if (req.args !== undefined) argv.push(`--args=${req.args}`);
  let child: ReturnType<typeof spawnDetached>;
  try {
    child = spawnDetached(argv, { cwd: root, env: supervisorEnv() });
  } catch (e) {
    throw new NotStarted(`cannot start the supervisor of \`${req.name}\`: ${(e as Error).message}`);
  }
  const until = Date.now() + START_WAIT_MS;
  for (;;) {
    let said: Started | undefined;
    try {
      said = readState<Started>(started);
    } catch {}
    if (said !== undefined) {
      if ("job" in said) return { id: said.job, bound: child.bound };
      throw new NotStarted(said.refused.why, said.refused);
    }
    if (!sameProcess(child.pid, child.started)) {
      let log = "";
      try {
        log = readFileSync(join(dir, SUPERVISOR_LOG), "utf8").trim();
      } catch {}
      throw new NotStarted(`the supervisor of \`${req.name}\` exited before it started: ${log}`);
    }
    if (Date.now() > until) throw new NotStarted(`the supervisor of \`${req.name}\` named no job within ${START_WAIT_MS / 1000} s`);
    await Bun.sleep(20);
  }
}

/** Asks launch `live` to stop, writing that `by` asked, and waits for its exit. Once `graceMs` and `STOP_SLACK_MS` have passed, its job is killed. Gives its exit, or null when it is lost. */
export async function stopLaunch(store: Store, live: Live, graceMs: number, by: string): Promise<JobExit | null> {
  askStop(store, live.id, by);
  const deadline = Date.now() + graceMs + STOP_SLACK_MS;
  let killed = false;
  for (;;) {
    const s = store.state(live.id, live.record);
    if (s.state === "exited") return s.exit;
    if (s.state === "lost") return null;
    if (Date.now() > deadline && !killed) {
      killed = true;
      store.kill(live.id, by);
    }
    await Bun.sleep(TICK_MS);
  }
}
