// One acceptance sweep at a time on this machine. A side run and the chain run work in trees of their own,
// but a sweep reaches past its tree: `examples/http.nvs` talks to the one origin on 127.0.0.1:8099, which a
// sweep holds up and closes when it is done, the Linux legs run in the one WSL distro, and the perf guards
// read a ratio off cores another sweep would be loading. Two sweeps at once turn each other's floor red, so
// `cmd/loop.ts`'s `sweepOver` takes this lock before its first check and gives it back after its last.
//
// The lock is the file `.agent-tmp/sweep.lock` under the main tree, which every worktree of the repository
// resolves to the same path. It is created only when absent, and it names the process that holds it and
// that process's tree. A sweep that finds it held waits, and tries again every `POLL_MS`. A file whose
// process has exited is stale, since a driver killed in a sweep leaves one, and the next sweep deletes it
// and takes the lock. `nv loop --run` does not take it: a session proving one check does not wait out
// another tree's whole sweep.
//
// What it spends: a sweep that finds another one running waits for it, up to that sweep's length.

import { closeSync, mkdirSync, openSync, readFileSync, rmSync, writeSync } from "node:fs";
import { dirname, join } from "node:path";
import { mainRoot } from "../lib/git.ts";
import { ROOT } from "../lib/paths.ts";

export const POLL_MS = 10_000;

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
