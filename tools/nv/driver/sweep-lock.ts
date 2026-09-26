// One sweep at a time on this machine among the sweeps that use a resource the machine has one of. A side
// run and the chain run work in trees of their own, but a sweep can reach past its tree: `examples/http.nvs`
// talks to the one origin on 127.0.0.1:8099, which a sweep holds up and closes when it is done; the Linux
// legs run in the one WSL distro; and the perf guards, the checks on the release profile, read a ratio off
// cores another sweep would be loading. Two such sweeps at once turn each other's floor red.
//
// `sharedResources` decides it from the checks a sweep reached and the legs it runs. A sweep that uses any
// of the three takes this lock before its first check and gives it back after its last: `cmd/loop.ts`'s
// `sweepOver`, and `select/full.ts`'s full run around the plan checks it runs with the origin up. Only a
// sweep that needs the origin holds it up. A sweep that uses none of them takes neither the lock nor the
// origin, so it never waits, and it never closes a listener that a sweep holding the lock is still using.
//
// The lock is the file `.agent-tmp/sweep.lock` under the main tree, which every worktree of the repository
// resolves to the same path. It is created only when absent, and it names the process that holds it and
// that process's tree. A sweep that finds it held waits, and tries again every `POLL_MS`. A file whose
// process has exited is stale, since a driver killed in a sweep leaves one, and the next sweep deletes it
// and takes the lock. `nv loop --run` does not take it: a session proving one check does not wait out
// another tree's whole sweep.
//
// What it spends: a sweep that uses a shared resource and finds another such sweep running waits for it, up
// to that sweep's length. A sweep that takes no lock still loads the cores under a locked sweep's perf
// guards.

import { closeSync, mkdirSync, openSync, readFileSync, rmSync, writeSync } from "node:fs";
import { dirname, join } from "node:path";
import { mainRoot } from "../lib/git.ts";
import { ROOT } from "../lib/paths.ts";
import { LEGS } from "../select/checks.ts";
import { type Check, isRelease, PROGRAM_KINDS } from "./accept.ts";
import { ORIGIN_PROGRAMS } from "./origin.ts";

export const POLL_MS = 10_000;

/** What a sweep uses that the machine has one of: `origin` to hold it up, `lock` to take the lock, and
 * `why`, one line per resource, for the sweep to print. */
export interface SharedNeed {
  origin: boolean;
  lock: boolean;
  why: string[];
}

/**
 * The shared resources a sweep uses, from `reached`, the checks it will start, and its legs: `runLegs`
 * when they run at all, and `legReached` for each one. A fixture of a program in `ORIGIN_PROGRAMS` needs
 * the origin and the lock; a Linux leg that runs and a check on the release profile need the lock.
 */
export function sharedResources(reached: Check[], runLegs: boolean, legReached: (leg: (typeof LEGS)[number]) => boolean): SharedNeed {
  const why: string[] = [];
  const talks = reached.filter((c) => PROGRAM_KINDS.has(c.kind) && c.file !== undefined && ORIGIN_PROGRAMS.includes(c.file));
  if (talks.length > 0) why.push(`the local origin, for ${talks.map((c) => c.file).join(", ")}`);
  const legs = runLegs ? LEGS.filter(legReached) : [];
  if (legs.length > 0) why.push(`the Linux legs: ${legs.join(", ")}`);
  const perf = reached.filter(isRelease);
  if (perf.length > 0) why.push(`the cores, for ${perf.length} perf guard(s) on the release profile`);
  return { origin: talks.length > 0, lock: why.length > 0, why };
}

/** The line a sweep prints for `need`: what took the lock, or that none did. */
export function needLine(need: SharedNeed): string {
  return need.lock ? `sweep lock: taken for ${need.why.join("; ")}` : "no shared resource: sweep lock not taken";
}

interface Holder {
  pid: number;
  root: string;
  since: string;
}

export interface SweepLock {
  release(): void;
}

function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return (e as NodeJS.ErrnoException).code === "EPERM";
  }
}

function holderOf(file: string): Holder | null {
  try {
    return JSON.parse(readFileSync(file, "utf8")) as Holder;
  } catch {
    return null;
  }
}

/** Creates `file` naming this process, or returns false when it already exists. */
function create(file: string): boolean {
  let fd: number;
  try {
    fd = openSync(file, "wx");
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "EEXIST") return false;
    throw e;
  }
  try {
    writeSync(fd, JSON.stringify({ pid: process.pid, root: ROOT, since: new Date().toISOString() } satisfies Holder));
  } finally {
    closeSync(fd);
  }
  return true;
}

/** The lock's path: `.agent-tmp/sweep.lock` under the main tree. */
export async function lockFile(): Promise<string> {
  return join(await mainRoot(), ".agent-tmp", "sweep.lock");
}

/**
 * Waits until this process holds the sweep lock at `file`, and returns the handle that gives it back.
 * `note` is told once, when the lock is held by another live process, whose sweep this one waits for.
 */
export async function takeSweepLock(o: { file?: string; pollMs?: number; note?: (text: string) => void } = {}): Promise<SweepLock> {
  const file = o.file ?? (await lockFile());
  mkdirSync(dirname(file), { recursive: true });
  let told = false;
  let unread = false;
  while (!create(file)) {
    const holder = holderOf(file);
    // A file still being written reads as nothing, and is read again on the next pass; one that still reads
    // as nothing a pass later was left half-written. A file naming this process is one it failed to give
    // back, since sweeps in one process never overlap.
    const stale: boolean = holder === null ? unread : holder.pid === process.pid || !alive(holder.pid);
    unread = holder === null && !stale;
    if (stale) {
      rmSync(file, { force: true });
      continue;
    }
    if (!told && holder !== null) {
      o.note?.(`sweep lock: waiting for the sweep in ${holder.root} (pid ${holder.pid}, since ${holder.since})`);
      told = true;
    }
    await Bun.sleep(o.pollMs ?? POLL_MS);
  }
  return {
    release() {
      if (holderOf(file)?.pid === process.pid) rmSync(file, { force: true });
    },
  };
}
