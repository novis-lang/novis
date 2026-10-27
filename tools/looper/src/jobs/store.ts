// The job store: one folder per job under the project's `.looper/jobs/`, which every runner writes and
// every reader lists without a server. It keeps the files of `crates/looper-job/src/store.rs`.
//
// A job's folder is named by its id, `<ms>-<pid>`: the start time in ms and the runner's pid. A job
// whose id has a folder, or whose Job Object name another job holds, takes the next free ms. So ids
// sort by time and never collide. It holds:
//
// - `job.json`, the `JobRecord`, written once the command's process exists;
// - the log, as `log.ts` says;
// - `exit`, the `JobExit`, written when the command ended. It is the only sign a job ended;
// - `killed`, the `Killed`, written by `Store.kill` before it ends the job;
// - `events.ndjson`, the job's progress events, which `docs/progress-format.md` defines;
// - `waiting`, the reason a process of the job waits on purpose, which the hang watch reads (`hang.ts`).
//
// Each file is written through tmp+rename (`../store/state.ts`), so a reader sees all of it or none.
// The job's Job Object is named after its id (`jobName`), so `Store.kill` reaches it from any process.
//
// A runner sets `PARENT_VAR` to its job's id for the command it runs. A job started below that command
// names it as its parent, so a list can show the jobs as a tree. A job names the loop its command runs
// in too, from that command's `LOOP_VAR`, so looper lists a loop's jobs by its name.
//
// A job is `running` while its runner runs, `exited` once `exit` is written, and `lost` when its runner
// ended without writing `exit`. A runner is matched by its pid and its start time, so a pid the OS
// gave out again does not keep a job running.
//
// `Store.sweep` keeps history inside `Bounds`: it deletes the jobs past the newest `Bounds.jobs` and
// those older than `Bounds.ageMs`, then the oldest ended jobs' logs while all logs together pass
// `Bounds.logBytes`. It never deletes a running job. `Store.sweepDue` runs it at most once per
// `SWEEP_EVERY_MS`, which the file `swept` records.
//
// # The bounds it keeps
//
// - A job's log is at most three parts of `LOG_PART` bytes.
// - History holds at most `Bounds.jobs` ended jobs, none older than `Bounds.ageMs`, and at most
//   `Bounds.logBytes` of logs, once a sweep has run.
// - A list or a sweep holds one entry per job folder, which the sweep bounds.

import { existsSync, mkdirSync, readdirSync, rmSync, rmdirSync, unlinkSync } from "node:fs";
import { join } from "node:path";
import { jobExists, killJob, started as startedAt } from "../os/index.ts";
import { readState, writeState } from "../store/state.ts";
import { DROPPED, HEAD, size, TAIL, TAIL_OLD } from "./log.ts";

/** The record of a job, in its folder's `job.json`. */
export const RECORD = "job.json";
/** The exit of a job, in its folder's `exit`. */
export const EXIT = "exit";
/** Who killed a job, in its folder's `killed`. */
export const KILLED = "killed";
/** A job's progress events. */
export const EVENTS = "events.ndjson";
/** The time of the last sweep, in the store's own folder. */
export const SWEPT = "swept";

/** The variable a runner sets to its job's id for the command it runs. */
export const PARENT_VAR = "LOOPER_JOB";
/** The variable that names the loop a process runs in, which a job records as its loop. */
export const LOOP_VAR = "LOOPER_LOOP";
/** The variable that names a job's event file to every process of the job. */
export const EVENTS_VAR = "LOOPER_EVENTS";
/** The variable that says whether a job's windows are hidden: `show` leaves them alone. */
export const WINDOWS_VAR = "LOOPER_WINDOWS";

/** The exit code a job ended by `Store.kill` gets. */
export const KILLED_CODE = 1;
/** The exit code a Bash call's job gets when the hang watch kills it (`hang.ts`): 124, as GNU `timeout` gives. */
export const HUNG_CODE = 124;

/** A folder with no record older than this is a runner that died before its command started: 1 day. */
const ORPHAN_MS = 24 * 60 * 60 * 1000;

/** How long a sweep holds before `sweepDue` runs the next: 10 minutes, as looper's server sweeps. */
export const SWEEP_EVERY_MS = 10 * 60 * 1000;

/** What ran the command. */
export type Kind = "bash" | "hook" | "check" | "verify" | "session" | "launch";

/** What a job ran, where, for whom and by which process: its folder's `job.json`. */
export interface JobRecord {
  kind: Kind;
  /** The root of the project the job ran in. */
  project: string | null;
  cwd: string;
  command: string;
  /** The Claude Code session that asked for it. */
  session: string | null;
  /** The subagent that asked for it. */
  agent: string | null;
  /** The tool call that asked for it. */
  tool_use: string | null;
  /** The pid of the process that holds the job and writes its log and its exit. */
  runner: number;
  /** The pid of the command's own process. */
  root: number;
  /** When the job started, in ms since the Unix epoch. */
  started: number;
  /** The job this one runs below, from `PARENT_VAR`. */
  parent?: string;
  /** The loop the job ran in, from the `LOOP_VAR` of the command's environment. Absent outside every loop. */
  loop?: string;
}

/** How a job ended: its folder's `exit`. */
export interface JobExit {
  code: number;
  /** When it ended, in ms since the Unix epoch. */
  ended: number;
}

/** Who killed a job and when: its folder's `killed`. */
export interface Killed {
  /** `ui`, `agent`, `cli:<who>` or `looper`. */
  by: string;
  at: number;
}

/** Where a job stands. */
export type JobState = { state: "running" } | { state: "exited"; exit: JobExit } | { state: "lost" };

/** One job a list found. */
export interface Entry {
  id: string;
  record: JobRecord;
  state: JobState;
}

/** How much history a sweep keeps. */
export interface Bounds {
  /** The most ended jobs kept. */
  jobs: number;
  /** The age past which an ended job goes, in ms. */
  ageMs: number;
  /** The most bytes of logs kept. */
  logBytes: number;
}

/** looper's bounds: 2,000 jobs, 7 days and 2 GiB of logs. */
export const BOUNDS: Bounds = { jobs: 2_000, ageMs: 7 * 24 * 60 * 60 * 1000, logBytes: 2 * 2 ** 30 };

/** What a sweep deleted: job folders whole, and logs of jobs that stay. */
export interface Swept {
  jobs: number;
  logs: number;
}

/** The name of the Job Object of job `id`, across the user's logon session. */
export function jobName(id: string): string {
  return `Local\\looper-job-${id}`;
}

/** The start time an id names, or null when the name is not an id. */
export function idMs(id: string): number | null {
  const m = /^(\d+)-(\d+)$/.exec(id);
  return m ? Number(m[1]) : null;
}

/** The store of the project at `root`: its `.looper/jobs/`. */
export function storeOf(root: string): Store {
  return new Store(join(root, ".looper", "jobs"));
}

/** The job folders of one project. */
export class Store {
  constructor(readonly root: string) {}

  /** The folder of job `id`. */
  dir(id: string): string {
    return join(this.root, id);
  }

  /** Makes the folder of a new job that starts at `started`, with an id whose job name is free. */
  create(started: number): string {
    mkdirSync(this.root, { recursive: true });
    for (let ms = started; ; ms++) {
      const id = `${ms}-${process.pid}`;
      try {
        mkdirSync(this.dir(id));
      } catch (e) {
        if ((e as NodeJS.ErrnoException).code === "EEXIST" && ms < started + 60_000) continue;
        throw e;
      }
      if (!jobExists(jobName(id))) return id;
      rmdirSync(this.dir(id));
    }
  }

  /** The record of job `id`, when it has one. */
  record(id: string): JobRecord | undefined {
    return readJson<JobRecord>(join(this.dir(id), RECORD));
  }

  /** Who killed job `id`, when someone did. */
  killed(id: string): Killed | undefined {
    return readJson<Killed>(join(this.dir(id), KILLED));
  }

  /** Where job `id`, whose record is `record`, stands now. */
  state(id: string, record: JobRecord): JobState {
    const exitPath = join(this.dir(id), EXIT);
    const exit = readJson<JobExit>(exitPath);
    if (exit) return { state: "exited", exit };
    const at = startedAt(record.runner);
    if (at !== null && at <= record.started) return { state: "running" };
    // The runner may have written `exit` between the two reads.
    const late = readJson<JobExit>(exitPath);
    return late ? { state: "exited", exit: late } : { state: "lost" };
  }

  /** The ids of every job folder, newest first. */
  ids(): string[] {
    let names: string[];
    try {
      names = readdirSync(this.root, { withFileTypes: true })
        .filter((e) => e.isDirectory())
        .map((e) => e.name);
    } catch {
      return [];
    }
    return names
      .map((id) => [idMs(id), id] as const)
      .filter((p): p is readonly [number, string] => p[0] !== null)
      .sort((a, b) => b[0] - a[0] || (b[1] < a[1] ? -1 : b[1] > a[1] ? 1 : 0))
      .map(([, id]) => id);
  }

  /** Every job with a record, newest first. */
  list(): Entry[] {
    const out: Entry[] = [];
    for (const id of this.ids()) {
      const record = this.record(id);
      if (record) out.push({ id, record, state: this.state(id, record) });
    }
    return out;
  }

  /** Ends running job `id` and its whole tree, after writing that `by` asked for it. Gives the record, or why not. */
  kill(id: string, by: string): JobRecord | string {
    const record = this.record(id);
    if (!record) return "no job has this id";
    const s = this.state(id, record);
    if (s.state === "exited") return `the job already exited with ${s.exit.code}`;
    if (s.state === "lost") return "the job is lost: its runner ended without recording an exit";
    if (!jobExists(jobName(id))) return "the job's Job Object is gone";
    writeState(join(this.dir(id), KILLED), { by, at: Date.now() } satisfies Killed);
    if (!killJob(jobName(id), KILLED_CODE)) return "the job's Job Object cannot be terminated";
    return record;
  }

  /** Deletes the history past `bounds` as of `now`, as the head comment says. */
  sweep(bounds: Bounds = BOUNDS, now = Date.now()): Swept {
    const swept: Swept = { jobs: 0, logs: 0 };
    const kept: { id: string; running: boolean }[] = [];
    let ended = 0;
    for (const id of this.ids()) {
      const dir = this.dir(id);
      const record = this.record(id);
      if (!record) {
        if (now - (idMs(id) ?? now) > ORPHAN_MS) {
          rmSync(dir, { recursive: true, force: true });
          swept.jobs++;
        }
        continue;
      }
      if (this.state(id, record).state === "running") {
        kept.push({ id, running: true });
        continue;
      }
      ended++;
      if (ended > bounds.jobs || now - record.started > bounds.ageMs) {
        rmSync(dir, { recursive: true, force: true });
        swept.jobs++;
      } else kept.push({ id, running: false });
    }
    let bytes = kept.reduce((sum, k) => sum + size(this.dir(k.id)), 0);
    for (const k of kept.toReversed()) {
      if (bytes <= bounds.logBytes) break;
      if (k.running) continue;
      const dir = this.dir(k.id);
      const bytesOf = size(dir);
      if (bytesOf === 0) continue;
      for (const part of [HEAD, TAIL_OLD, TAIL, DROPPED]) {
        if (existsSync(join(dir, part))) unlinkSync(join(dir, part));
      }
      bytes -= bytesOf;
      swept.logs++;
    }
    return swept;
  }

  /** Sweeps with `bounds` when the last sweep is `SWEEP_EVERY_MS` old, or there was none. Gives what it swept. */
  sweepDue(bounds: Bounds = BOUNDS, now = Date.now()): Swept | null {
    const path = join(this.root, SWEPT);
    const last = readJson<number>(path);
    if (typeof last === "number" && now - last < SWEEP_EVERY_MS && last <= now) return null;
    mkdirSync(this.root, { recursive: true });
    writeState(path, now);
    return this.sweep(bounds, now);
  }
}

/** The JSON document at `path`, or undefined when it is missing or does not parse. */
function readJson<T>(path: string): T | undefined {
  try {
    return readState<T>(path);
  } catch {
    return undefined;
  }
}
