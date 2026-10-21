// Watching a running program for a hang, so a stuck test or tool is stopped within minutes instead of at
// its time limit, and the session that ran it is told what hung.
//
// Every `EVERY_MS` the watch reads what the program's tree has used (`Tree.usage`) and how much it has
// printed. A sample is progress when the tree printed anything, moved at least `IO_STEP` bytes through
// files, pipes or sockets, or started or lost a process. Without progress the tree is one of two things:
//
// - **Stalled**: it also used less than `BUSY_SHARE` of one CPU. Every process waits: a deadlock, a read
//   from a pipe or socket nobody writes, a lock nobody frees. This is certain early, and confirmed after
//   `stallMs` of it.
// - **Spinning**: it uses the CPU and shows nothing else. A loop that does not end looks like this, but
//   so does a long silent computation, so it is confirmed only after `spinMs`.
//
// Half way to either bound the run is suspected: the watch then reads every process of the tree
// (`Tree.members`) and samples every `CLOSE_EVERY_MS`, so the report can say which process used the CPU
// and which waited. Any progress clears the suspicion. A confirmed hang kills the tree through
// `Tree.kill`, the same kill a time limit makes, so it reaches only processes this run started. The
// report goes to `.agent-tmp/hangs/` and to stderr, and names the processes, the test libtest last said
// was running long, and the last lines the program printed.
//
// A tree with a WSL or Docker client in it (`OUTSIDE`) does its work where `usage` cannot read it, so its
// idle CPU says nothing. Once the first close look finds one, only output counts: a tree that prints
// nothing for `spinMs` is **silent**, and that is its hang.
//
// A watch that cannot see the output, because it goes straight to the terminal, sets `ioStep` to 1: the
// writes are then its only sign of output, so any byte counts.
//
// The time limit a run already has stays: the watch only ends a run earlier. A program that waits on
// purpose for longer than `stallMs` (a container pull, a service coming up) passes a longer `stallMs`,
// or is not watched.
//
// What it costs: per watched run one timer and one `usage` read every `EVERY_MS`, and the last
// `TAIL_BYTES` of its output, which `proc.run` keeps anyway; a suspected run one `members` read per close
// sample.

import { mkdirSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { ROOT } from "./paths.ts";
import type { Member, Tree, Usage } from "./proctree.ts";

export interface HangOptions {
  /** Milliseconds a tree may wait with no CPU and no output before it is a hang. */
  stallMs?: number;
  /** Milliseconds a tree may use the CPU with no output and no I/O before it is a hang. */
  spinMs?: number;
  /** Bytes read or written in one sample that count as progress; `IO_STEP` unless the watch cannot see the output. */
  ioStep?: number;
  /** Milliseconds between two samples; `EVERY_MS` unless a test needs a quicker one. */
  everyMs?: number;
}

export const STALL_MS = 90_000;
export const SPIN_MS = 10 * 60_000;
export const EVERY_MS = 5_000;
export const CLOSE_EVERY_MS = 1_000;
/** Bytes read or written in one sample that count as progress: a poll of a small file every 100 ms does not. */
export const IO_STEP = 64 * 1024;
/** The share of one CPU a tree must use in a sample to count as busy. */
export const BUSY_SHARE = 0.02;
const TAIL_LINES = 40;
export const TAIL_BYTES = 16 * 1024;

export type Verdict = "ok" | "suspect" | "stalled" | "spinning" | "silent";

/** Programs whose work runs outside the tree, in the WSL VM or Docker's engine, so its CPU and I/O cannot be read. */
const OUTSIDE = new Set(["wsl.exe", "wslhost.exe", "wslrelay.exe", "docker.exe", "docker", "docker-compose.exe", "docker-compose", "podman.exe", "podman"]);

/** Whether a tree with `members` does its work where its CPU cannot be read. */
export const worksOutside = (members: Member[]) => members.some((m) => OUTSIDE.has(m.name.toLowerCase()));

/** The decision alone, fed one sample at a time, so tests can drive it with made-up numbers. */
export class Judge {
  private last: Usage;
  private out: number;
  private at: number;
  private progressAt: number;
  private idleSince: number | null = null;
  /** Set once the tree is known to work outside itself: then only its output counts. */
  outside = false;

  constructor(
    private readonly stallMs: number,
    private readonly spinMs: number,
    first: Usage,
    out: number,
    now: number,
    private readonly ioStep = IO_STEP,
  ) {
    this.last = first;
    this.out = out;
    this.at = now;
    this.progressAt = now;
  }

  /** Takes the sample `u`, with `out` bytes printed so far, read at `now`. */
  step(u: Usage, out: number, now: number): Verdict {
    const dt = Math.max(1, now - this.at);
    const progress = out !== this.out || u.procs !== this.last.procs || u.ioBytes - this.last.ioBytes >= this.ioStep;
    const busy = u.cpuMs - this.last.cpuMs >= BUSY_SHARE * dt;
    this.last = u;
    this.out = out;
    this.at = now;
    if (progress) {
      this.progressAt = now;
      this.idleSince = null;
      return "ok";
    }
    if (busy) this.idleSince = null;
    else this.idleSince ??= now - dt;
    const idle = this.idleSince === null ? 0 : now - this.idleSince;
    const quiet = now - this.progressAt;
    if (this.outside) return quiet >= this.spinMs ? "silent" : quiet >= this.spinMs / 2 ? "suspect" : "ok";
    if (idle >= this.stallMs) return "stalled";
    if (quiet >= this.spinMs) return "spinning";
    return idle >= this.stallMs / 2 || quiet >= this.spinMs / 2 ? "suspect" : "ok";
  }

  /** Milliseconds since the last progress. */
  quietMs(now: number): number {
    return now - this.progressAt;
  }
}

/** What a watched run hands the watch: its output so far, as a byte count and as its last lines. */
export interface Printed {
  bytes(): number;
  tail(): string;
}

/** The last test libtest said was running long in `text`, or null. */
export function longTest(text: string): string | null {
  const all = [...text.matchAll(/^test (\S+) has been running for over \d+ seconds/gm)];
  return all.at(-1)?.[1] ?? null;
}

/** The report of a hang: what hung, how, which process did what in the close look, and the last output. */
export function report(argv: string[], verdict: "stalled" | "spinning" | "silent", quietMs: number, before: Member[], after: Member[], spanMs: number, tail: string, path: string | null): string {
  const s = Math.round(quietMs / 1000);
  const how =
    verdict === "stalled"
      ? `waited ${s} s with no output, no file or pipe traffic and no CPU time: every process is blocked`
      : verdict === "spinning"
        ? `used the CPU for ${s} s with no output and no file or pipe traffic: a loop that does not end, or a far slower run`
        : `printed nothing for ${s} s, and its work runs in WSL or Docker, where its CPU cannot be read`;
  const lines = [`HANG: \`${argv.join(" ")}\` ${how}. It was killed.`];
  if (path !== null) lines.push(`report: ${path}`);
  const test = longTest(tail);
  if (test !== null) lines.push(`libtest's last long-running test: ${test}`);
  const was = new Map(before.map((m) => [m.pid, m.cpuMs]));
  lines.push(`processes, with the CPU each used in the last ${Math.round(spanMs / 1000)} s:`);
  for (const m of after.length > 0 ? after : before) {
    const used = was.has(m.pid) ? `${Math.round(m.cpuMs - was.get(m.pid)!)} ms` : "started since";
    lines.push(`  ${m.pid} ${m.name}: ${used}, ${Math.round(m.cpuMs)} ms in all`);
  }
  const last = tail.trim() === "" ? [] : tail.replace(/\r\n/g, "\n").trimEnd().split("\n").slice(-TAIL_LINES);
  if (last.length === 0) lines.push("it printed nothing");
  else lines.push(`its last ${last.length} lines of output:`, ...last.map((l) => `  ${l}`));
  return `${lines.join("\n")}\n`;
}

/** Writes the report `write(path)` gives under `.agent-tmp/hangs/`, and returns it; without its path when it cannot be written. */
function save(argv: string[], write: (path: string | null) => string): string {
  const name = (argv[0] ?? "run").replace(/^.*[\\/]/, "").replace(/[^\w.-]/g, "_");
  const file = join(ROOT, ".agent-tmp", "hangs", `${new Date().toISOString().replace(/[:.]/g, "-")}-${name}.txt`);
  const text = write(relative(ROOT, file).replaceAll("\\", "/"));
  try {
    mkdirSync(join(ROOT, ".agent-tmp", "hangs"), { recursive: true });
    writeFileSync(file, text);
    return text;
  } catch {
    return write(null);
  }
}

/** A running watch. `stop` ends it; `hung` is the report once it has killed the tree. */
export interface Watching {
  stop(): void;
  readonly hung: string | null;
}

/**
 * Watches `tree` until `stop`. On a confirmed hang it kills the tree, writes the report, prints it on
 * stderr and keeps it as `hung`. A tree whose usage cannot be read is not watched.
 */
export function watch(argv: string[], tree: Tree, printed: Printed, opts: HangOptions = {}): Watching {
  const state = { hung: null as string | null };
  const first = tree.usage();
  if (first === null) return { stop: () => {}, get hung() { return state.hung; } };
  const judge = new Judge(opts.stallMs ?? STALL_MS, opts.spinMs ?? SPIN_MS, first, printed.bytes(), performance.now(), opts.ioStep);
  let look: { members: Member[]; at: number } | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  const tick = () => {
    const u = tree.usage();
    if (u === null) return;
    const now = performance.now();
    const v = judge.step(u, printed.bytes(), now);
    if (v === "ok") look = null;
    else if (look === null) {
      look = { members: tree.members(), at: now };
      // A stall verdict reached before this look is dropped too: the CPU it read was never the work's.
      judge.outside ||= worksOutside(look.members);
    }
    if (v === "silent" || ((v === "stalled" || v === "spinning") && !judge.outside)) {
      const after = tree.members();
      tree.kill();
      const tail = printed.tail();
      state.hung = save(argv, (path) => report(argv, v, judge.quietMs(now), look!.members, after, now - look!.at, tail, path));
      // CI keeps no `.agent-tmp/`, so there the whole report goes to stderr.
      process.stderr.write(`nv: ${process.env.CI ? state.hung : `${state.hung.split("\n").slice(0, 2).join(" ")}\n`}`);
      return;
    }
    timer = setTimeout(tick, look === null ? every : Math.min(every, CLOSE_EVERY_MS));
  };
  const every = opts.everyMs ?? EVERY_MS;
  timer = setTimeout(tick, every);
  return {
    stop: () => {
      if (timer !== null) clearTimeout(timer);
      timer = null;
    },
    get hung() {
      return state.hung;
    },
  };
}
