// The hang watch on a Bash call's job: a call that waits or spins with nothing to show is killed within
// minutes, instead of at the harness's own time limit, and the session is told what hung.
//
// Every `EVERY_MS` the watch reads what the job has used (`Tree.usage`) and how many bytes it has
// printed. A sample is progress when the job printed anything, read or wrote at least `IO_STEP` bytes,
// or started or lost a process. Without progress the job is one of two things:
//
// - **Stalled**: it also used less than `BUSY_SHARE` of one CPU. Every process waits: a deadlock, a read
//   nobody answers, a lock nobody frees. It is confirmed after `stallMs` of it.
// - **Spinning**: it uses the CPU and shows nothing else. A loop that does not end looks like this, and
//   so does a long silent computation, so it is confirmed only after `spinMs`.
//
// Half way to either bound the job is suspected. The watch then first asks whether the job waits on
// purpose (`held`), and clears the suspicion when it does:
//
// - a process of the job listens on a TCP port, as a server started in the background does;
// - the job's last line of output is one of `WAIT_LINES`, as cargo's wait for another build's lock;
// - a process of the job refreshed the job's `WAITING` file within `WAIT_FRESH_MS`. A kit command that
//   waits on looper's service or on another job calls `waitOnPurpose` for as long as it waits.
//
// A suspicion that stays makes the watch read every process of the job (`Tree.members`) and sample every
// `CLOSE_EVERY_MS`, so the report can say which process used the CPU and which waited. Any progress
// clears it.
//
// A confirmed hang kills the job through `Tree.kill` with `HUNG_CODE`. That is the call's own Job
// Object, the same kill the wrapper makes when Claude Code exits, so it reaches only processes this call
// started. The report goes to the project's `hangs/` folder under its scratch directory, at most
// `MAX_REPORTS` of them. It names the command, the processes, the test libtest last said was running
// long, and the last `TAIL_LINES` lines the call printed.
//
// `exempt` names the commands the wrapper does not watch: those `EXEMPT` names, which follow or watch
// something on purpose, and those the project's `[jobs] hang_exempt` names.
//
// # What it costs
//
// Per watched call one timer and one job query every `EVERY_MS`, and the last `TAIL_BYTES` of its output.
// A suspected call adds one port table read and one `stat` per sample, and one `members` read once.

import { mkdirSync, readdirSync, statSync, unlinkSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { ruleRegex } from "../goal/config.ts";
import { listeners, type Member, type Tree, type Usage } from "../os/index.ts";
import { EVENTS_VAR, HUNG_CODE } from "./store.ts";

/** How long a job may wait with no CPU and no output before it is a hang, without `[jobs] hang_stall_s`. */
export const STALL_MS = 90_000;
/** How long a job may use the CPU with no output before it is a hang, without `[jobs] hang_spin_s`. */
export const SPIN_MS = 10 * 60_000;
/** How often the watch samples a job. */
export const EVERY_MS = 5_000;
/** How often the watch samples a suspected job. */
export const CLOSE_EVERY_MS = 1_000;
/** Bytes read or written in one sample that count as progress: a poll of a small file every 100 ms does not. */
export const IO_STEP = 64 * 1024;
/** The share of one CPU a job must use in a sample to count as busy. */
export const BUSY_SHARE = 0.02;
/** The most output lines a report quotes. */
export const TAIL_LINES = 40;
/** The most output bytes the watch keeps for its report. */
export const TAIL_BYTES = 16 * 1024;
/** The file in a job's folder that says a process of the job waits on purpose. */
export const WAITING = "waiting";
/** How often `waitOnPurpose` refreshes the job's `WAITING` file. */
export const WAIT_BEAT_MS = 20_000;
/** How old a `WAITING` file may be and still hold the job. */
export const WAIT_FRESH_MS = 2 * WAIT_BEAT_MS;
/** The most reports the `hangs/` folder keeps. Writing one deletes the oldest past it. */
export const MAX_REPORTS = 32;

/** Commands that follow or watch something on purpose, in `ruleRegex` syntax. */
export const EXEMPT = [String.raw`\btail\b[^|;&]*\s(-[A-Za-z]*[fF]|--follow)`, String.raw`(^|\s)--watch\b`, String.raw`\bdocker\b.*\s(up|logs|attach|wait|events)(\s|$)`];

/** Last lines that say the job waits for something outside it that will come. */
export const WAIT_LINES = [/Blocking waiting for file lock/];

export type Verdict = "ok" | "suspect" | "stalled" | "spinning";

/** The decision alone, fed one sample at a time, so tests can drive it with made-up numbers. A bound of 0 never confirms. */
export class Judge {
  private last: Usage;
  private out: number;
  private at: number;
  private progressAt: number;
  private idleSince: number | null = null;

  constructor(
    private readonly stallMs: number,
    private readonly spinMs: number,
    first: Usage,
    out: number,
    now: number,
  ) {
    this.last = first;
    this.out = out;
    this.at = now;
    this.progressAt = now;
  }

  /** Takes the sample `u`, with `out` bytes printed so far, read at `now`. */
  step(u: Usage, out: number, now: number): Verdict {
    const dt = Math.max(1, now - this.at);
    const progress = out !== this.out || u.procs !== this.last.procs || u.ioBytes - this.last.ioBytes >= IO_STEP;
    const busy = u.cpuMs - this.last.cpuMs >= BUSY_SHARE * dt;
    this.last = u;
    this.out = out;
    this.at = now;
    if (progress) {
      this.clear(now);
      return "ok";
    }
    if (busy) this.idleSince = null;
    else this.idleSince ??= now - dt;
    const idle = this.idleSince === null ? 0 : now - this.idleSince;
    const quiet = now - this.progressAt;
    const stall = this.stallMs > 0;
    const spin = this.spinMs > 0;
    if (stall && idle >= this.stallMs) return "stalled";
    if (spin && quiet >= this.spinMs) return "spinning";
    return (stall && idle >= this.stallMs / 2) || (spin && quiet >= this.spinMs / 2) ? "suspect" : "ok";
  }

  /** Counts `now` as progress: the job waits on purpose. */
  clear(now: number): void {
    this.progressAt = now;
    this.idleSince = null;
  }

  /** Milliseconds since the last progress. */
  quietMs(now: number): number {
    return now - this.progressAt;
  }
}

/** A job's output as the watch sees it: how many bytes it printed, and its last `TAIL_BYTES`. */
export class Printed {
  bytes = 0;
  private last: Buffer = Buffer.alloc(0);

  add(chunk: Uint8Array): void {
    this.bytes += chunk.length;
    const keep = Math.max(0, TAIL_BYTES - chunk.length);
    this.last = Buffer.concat([this.last.subarray(Math.max(0, this.last.length - keep)), chunk.subarray(Math.max(0, chunk.length - TAIL_BYTES))]);
  }

  /** The last bytes as text, without colour codes. */
  tail(): string {
    return this.last.toString("utf8").replace(/\x1b\[[0-9;]*[A-Za-z]/g, "");
  }

  /** The last line that is not blank. */
  lastLine(): string {
    const lines = this.tail().split(/\r?\n|\r/).filter((l) => l.trim() !== "");
    return lines.at(-1) ?? "";
  }
}

/** Whether the watch leaves `command` alone: `EXEMPT` or one of the project's `patterns` finds it. */
export function exempt(command: string, patterns: readonly string[]): boolean {
  return [...EXEMPT, ...patterns].some((p) => {
    try {
      return ruleRegex(p).test(command);
    } catch {
      return false;
    }
  });
}

/** The last test libtest said was running long in `text`, or null. */
export function longTest(text: string): string | null {
  const all = [...text.matchAll(/^test (\S+) has been running for over \d+ seconds/gm)];
  return all.at(-1)?.[1] ?? null;
}

/** `command` on one line, at most 200 characters. */
function oneLine(command: string): string {
  const flat = command.replace(/\s+/g, " ").trim();
  return flat.length > 200 ? `${flat.slice(0, 199)}…` : flat;
}

/** The report of a hang: what hung and how, which process did what in the close look, and the last output. */
export function report(command: string, verdict: "stalled" | "spinning", quietMs: number, before: Member[], after: Member[], spanMs: number, tail: string, path: string | null): string {
  const s = Math.round(quietMs / 1000);
  const how =
    verdict === "stalled"
      ? `waited ${s} s with no output, no file or pipe traffic and no CPU time: every process is blocked`
      : `used the CPU for ${s} s with no output and no file or pipe traffic: a loop that does not end, or a far slower run`;
  const lines = [`HANG: \`${oneLine(command)}\` ${how}. It was killed.`];
  if (path !== null) lines.push(`report: ${path}`);
  const test = longTest(tail);
  if (test !== null) lines.push(`libtest's last long-running test: ${test}`);
  const was = new Map(before.map((m) => [m.pid, m.cpuMs]));
  lines.push(`processes, with the CPU each used in the last ${Math.round(spanMs / 1000)} s:`);
  for (const m of after.length > 0 ? after : before) {
    const then = was.get(m.pid);
    const used = then === undefined ? "started since" : `${Math.round(m.cpuMs - then)} ms`;
    lines.push(`  ${m.pid} ${m.name}: ${used}, ${Math.round(m.cpuMs)} ms in all`);
  }
  const last = tail.trim() === "" ? [] : tail.replace(/\r\n/g, "\n").trimEnd().split("\n").slice(-TAIL_LINES);
  if (last.length === 0) lines.push("it printed nothing");
  else lines.push(`its last ${last.length} lines of output:`, ...last.map((l) => `  ${l}`));
  return `${lines.join("\n")}\n`;
}

/** Writes the report `write(path)` gives into `dir`, keeps the newest `MAX_REPORTS`, and returns it. Without its path when it cannot be written. */
function save(dir: string, write: (path: string | null) => string): string {
  const file = join(dir, `${new Date().toISOString().replace(/[:.]/g, "-")}-looper-sh.txt`);
  const text = write(file.replaceAll("\\", "/"));
  try {
    mkdirSync(dir, { recursive: true });
    writeFileSync(file, text);
  } catch {
    return write(null);
  }
  try {
    const all = readdirSync(dir).filter((n) => n.endsWith("-looper-sh.txt")).sort();
    for (const old of all.slice(0, Math.max(0, all.length - MAX_REPORTS))) unlinkSync(join(dir, old));
  } catch {}
  return text;
}

/**
 * Says that this process waits on purpose, for the job `LOOPER_EVENTS` names in `env`, until the stop
 * it gives is called. It writes the job's `WAITING` file now and every `WAIT_BEAT_MS`. Outside a job it
 * does nothing.
 */
export function waitOnPurpose(why: string, env: Record<string, string | undefined> = process.env): () => void {
  const events = env[EVENTS_VAR];
  if (events === undefined || events === "") return () => {};
  const path = join(dirname(events), WAITING);
  const beat = () => {
    try {
      writeFileSync(path, `${why}\n`);
    } catch {}
  };
  beat();
  const timer = setInterval(beat, WAIT_BEAT_MS);
  timer.unref();
  return () => {
    clearInterval(timer);
    try {
      unlinkSync(path);
    } catch {}
  };
}

/** Whether a process of the job in `jobDir` refreshed its `WAITING` file within `WAIT_FRESH_MS`. */
function waitingFile(jobDir: string): boolean {
  try {
    return Date.now() - statSync(join(jobDir, WAITING)).mtimeMs <= WAIT_FRESH_MS;
  } catch {
    return false;
  }
}

/** Whether a process of `tree` listens on a TCP port. */
function listens(tree: Tree): boolean {
  try {
    const pids = new Set(tree.pids());
    return listeners().some(([, pid]) => pids.has(pid));
  } catch {
    return false;
  }
}

/** Why the job waits on purpose, or null. The cheapest sign is read first. */
export function held(tree: Tree, jobDir: string, printed: Printed): string | null {
  const line = printed.lastLine();
  if (WAIT_LINES.some((w) => w.test(line))) return "its last line says it waits";
  if (waitingFile(jobDir)) return "a process of it waits on purpose";
  if (listens(tree)) return "a process of it listens on a port";
  return null;
}

export interface WatchOptions {
  /** The command as the report names it. */
  command: string;
  /** The job's folder, which may hold its `WAITING` file. */
  jobDir: string;
  /** The folder the report goes to. */
  reportDir: string;
  /** Milliseconds a job may wait with no CPU and no output; 0 never. */
  stallMs: number;
  /** Milliseconds a job may use the CPU with no output; 0 never. */
  spinMs: number;
  /** Milliseconds between two samples; `EVERY_MS` unless a test needs a quicker one. */
  everyMs?: number;
  /** Called once a hang is confirmed, just before the kill. */
  killing?: () => void;
}

/** A running watch. `stop` ends it, and `hung` is the report once it has killed the job. */
export interface Watching {
  stop(): void;
  readonly hung: string | null;
}

/**
 * Watches `tree` until `stop`, as the head comment says. On a confirmed hang it kills the job with
 * `HUNG_CODE`, writes the report and keeps it as `hung`. A job whose usage cannot be read is not watched.
 */
export function watch(tree: Tree, printed: Printed, opts: WatchOptions): Watching {
  const state = { hung: null as string | null };
  const done: Watching = {
    stop: () => {},
    get hung() {
      return state.hung;
    },
  };
  const first = tree.usage();
  if (first === null || (opts.stallMs <= 0 && opts.spinMs <= 0)) return done;
  const every = opts.everyMs ?? EVERY_MS;
  const judge = new Judge(opts.stallMs, opts.spinMs, first, printed.bytes, performance.now());
  let look: { members: Member[]; at: number } | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  const tick = () => {
    timer = null;
    try {
      sample();
    } catch {
      // A sample that fails ends the watch, and the job runs on unwatched.
    }
  };
  const sample = () => {
    const u = tree.usage();
    if (u === null) return;
    const now = performance.now();
    let v = judge.step(u, printed.bytes, now);
    if (v !== "ok" && held(tree, opts.jobDir, printed) !== null) {
      judge.clear(now);
      v = "ok";
    }
    if (v === "ok") look = null;
    else look ??= { members: tree.members(), at: now };
    if (look !== null && (v === "stalled" || v === "spinning")) {
      const verdict = v;
      const { members, at } = look;
      const after = tree.members();
      opts.killing?.();
      try {
        tree.kill(HUNG_CODE);
      } catch {
        return;
      }
      const tail = printed.tail();
      state.hung = save(opts.reportDir, (path) => report(opts.command, verdict, judge.quietMs(now), members, after, now - at, tail, path));
      return;
    }
    timer = setTimeout(tick, look === null ? every : Math.min(every, CLOSE_EVERY_MS));
  };
  timer = setTimeout(tick, every);
  done.stop = () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  };
  return done;
}
