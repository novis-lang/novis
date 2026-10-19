// The guard's pending entries: one line for each Bash call the guard let through, which the shell
// wrapper (`sh.ts`) takes to learn the subagent and the tool call behind its command. It keeps the
// rules of `crates/looper-job/src/pending.rs`.
//
// A session's entries are JSON lines in the project's `.looper/pending/<session>.jsonl`. `push` appends
// one, and `take` removes the newest entry whose command equals the wrapper's. Each rewrites the file
// through a tmp file of its own process and a rename. `sweep` deletes a file untouched for `IDLE_MS`.
//
// # The bounds it keeps
//
// - An entry older than `MAX_AGE_MS` is dropped on every rewrite, and `take` never matches one.
// - A session's file holds at most `MAX_ENTRIES` entries, the newest.
// - A command longer than `MAX_COMMAND` bytes is not written, so a file stays under 5 MiB.
//
// # Known gaps
//
// - Two calls of one session that rewrite its file at once can lose one entry, whose job then names its
//   session only.

import { closeSync, mkdirSync, openSync, readdirSync, readFileSync, renameSync, statSync, unlinkSync, writeSync } from "node:fs";
import { join } from "node:path";
import { RETRIES, RETRY_MS } from "../store/state.ts";

/** The age past which an entry is dropped: 60 s. */
export const MAX_AGE_MS = 60_000;
/** The most entries a session's file holds. */
export const MAX_ENTRIES = 64;
/** The longest command an entry holds: 64 KiB. */
export const MAX_COMMAND = 64 << 10;
/** How long a file goes untouched before `sweep` deletes it: 1 day. */
export const IDLE_MS = 24 * 60 * 60 * 1000;

/** One Bash call the guard let through. */
export interface Pending {
  session_id: string;
  agent_id: string | null;
  tool_use_id: string | null;
  command: string;
  /** When the guard saw it, in ms since the Unix epoch. */
  at: number;
}

/** The pending folder of the project at `root`. */
export function pendingDir(root: string): string {
  return join(root, ".looper", "pending");
}

/** The file of `session` in the pending folder `dir`, or null when the id cannot name a file. */
function file(dir: string, session: string): string | null {
  return /^[A-Za-z0-9_-]{1,128}$/.test(session) ? join(dir, `${session}.jsonl`) : null;
}

/** The entries in `path` younger than `MAX_AGE_MS` at `now`, oldest first. */
function fresh(path: string, now: number): Pending[] {
  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch {
    return [];
  }
  const out: Pending[] = [];
  for (const line of text.split("\n")) {
    try {
      const e = JSON.parse(line) as Pending;
      if (typeof e.command === "string" && typeof e.at === "number" && now - e.at <= MAX_AGE_MS) out.push(e);
    } catch {}
  }
  return out;
}

/** Replaces `path` with `entries`, or deletes it when there are none. */
function rewrite(path: string, entries: Pending[]): void {
  if (entries.length === 0) {
    try {
      unlinkSync(path);
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
    }
    return;
  }
  const tmp = `${path}.${process.pid}.tmp`;
  const fd = openSync(tmp, "w");
  try {
    writeSync(fd, entries.map((e) => `${JSON.stringify(e)}\n`).join(""));
  } finally {
    closeSync(fd);
  }
  for (let attempt = 1; ; attempt++) {
    try {
      renameSync(tmp, path);
      return;
    } catch (e) {
      if (attempt >= RETRIES) {
        try {
          unlinkSync(tmp);
        } catch {}
        throw e;
      }
      Bun.sleepSync(RETRY_MS);
    }
  }
}

/** Appends `entry` to its session's file in the pending folder `dir`, as of `now`. */
export function push(dir: string, entry: Pending, now = Date.now()): void {
  if (Buffer.byteLength(entry.command) > MAX_COMMAND) return;
  const path = file(dir, entry.session_id);
  if (path === null) throw new Error("the session id cannot name a file");
  mkdirSync(dir, { recursive: true });
  const entries = [...fresh(path, now), entry];
  rewrite(path, entries.slice(Math.max(0, entries.length - MAX_ENTRIES)));
}

/** Removes and gives the newest entry of `session` in the pending folder `dir` whose command is `command`, as of `now`. */
export function take(dir: string, session: string, command: string, now = Date.now()): Pending | null {
  const path = file(dir, session);
  if (path === null) return null;
  const entries = fresh(path, now);
  const at = entries.findLastIndex((e) => e.command === command);
  if (at < 0) return null;
  const [entry] = entries.splice(at, 1);
  try {
    rewrite(path, entries);
  } catch {}
  return entry ?? null;
}

/** Deletes every pending file in `dir` untouched for `IDLE_MS` at `now`, and gives how many it deleted. */
export function sweep(dir: string, now = Date.now()): number {
  let names: string[];
  try {
    names = readdirSync(dir);
  } catch {
    return 0;
  }
  let deleted = 0;
  for (const name of names) {
    const path = join(dir, name);
    try {
      if (now - statSync(path).mtimeMs > IDLE_MS) {
        unlinkSync(path);
        deleted++;
      }
    } catch {}
  }
  return deleted;
}
