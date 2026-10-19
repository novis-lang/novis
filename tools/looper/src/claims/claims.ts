// Edit claims: which session has uncommitted edits in which file, so two sessions never edit or commit
// one file at once. It ports `crates/looper-core/src/claims.rs` and the `Machine` of
// `crates/looper-engine/src/claims.rs`.
//
// # What it is
//
// - A claim is one file with uncommitted edits by one session. The guard records it at the session's
//   first `Edit`, `Write` or `NotebookEdit` of the file (`claim`). It holds the tree root, the path from
//   that root, the agent that made the edit and the time. The session's `Holder` names its Claude Code
//   process, its loop and its goal.
// - A project's claims live in its data folder, `.looper/claims/`, one `<session_id>.json` per session.
//   A session's file is a state file, rewritten while its `<session_id>.lock` is held. So two parallel
//   edits of one session lose no claim, and a session that takes over or releases another's claim
//   rewrites that file under its lock.
//
// # The rule
//
// - An edit of a file another live session claims is refused, with the reason `claim` returns.
// - A session's own claim never refuses it. Its subagents share its session id, so theirs never do.
// - A claim is released once the file's working content equals `HEAD`'s again. The release is lazy: a
//   claim's file is checked only when the claim would refuse an edit or a commit, or is listed.
// - A session is live while its Claude Code process runs, the same pid with the same start time
//   (`Probe.started`). A holder without them is live while its loop's worker runs, which `release`s its
//   claims at the session's end. An interactive holder without them is live until `QUIET_MS` after its
//   newest edit.
// - A dead session's claim never refuses an edit. The first edit by another session takes it over, and
//   the new claim names the dead holder in `taken_from`.
// - A commit never stages a file another live session claims. `conflict` names the first such file
//   among those a `git add` or a wrap would stage.
//
// # Listing and releasing
//
// - `listing` reads every claim of the project with its session and whether that session is live. It
//   first releases each claim whose file is clean, so a committed file is never listed.
// - `releaseAsked` frees a session's claims for whoever asked, and records them in
//   `<session_id>.released`. An `agent` may release only a dead session's claims.
// - `release` frees them unasked, as the driver does when a loop's session ends.
//
// # The bounds it keeps
//
// - A session holds at most `MAX_CLAIMS` claims. Past that, an edit of a new file is refused, once the
//   session's own claims on clean files are dropped.
// - `sweep` deletes a session file with no claims left, and any session file untouched for `IDLE_MS`,
//   with its release record. A session has at most one release record, the newest.
// - An edit with no conflict reads one small file per session of the project, and writes its own.
// - A listing runs `git status` once per tree that holds a claim.
//
// # Known gaps
//
// - A file edited by a shell command, not a tool, is not claimed.
// - Two sessions whose first edits of one file land at the same instant can both claim it, because each
//   reads the others' files without their locks.
// - A listing can release a claim between its session's guard passing an edit of a clean file and the
//   edit landing. The file is then unclaimed until that session edits it again.

import { closeSync, existsSync, mkdirSync, openSync, readdirSync, rmSync, statSync } from "node:fs";
import { join } from "node:path";
import { started } from "../os/index.ts";
import { readState, writeState } from "../store/state.ts";

/** The folder of the claim files, under the project's data folder. */
export const DIR = "claims";
/** The most claims one session holds. */
export const MAX_CLAIMS = 2_000;
/** How long a session file goes untouched before `sweep` deletes it: 7 days. */
export const IDLE_MS = 7 * 24 * 60 * 60 * 1000;
/** How long after its newest edit an interactive holder with no process is live: 2 hours. */
export const QUIET_MS = 2 * 60 * 60 * 1000;
/** How long a lock is waited for before the edit goes unjudged. */
const LOCK_WAIT_MS = 5_000;
/** The age past which a lock file is a crashed guard's, and is removed. */
const LOCK_STALE_MS = 30_000;
/** The pause between two tries at a held lock. */
const LOCK_RETRY_MS = 10;

/** The session that holds a set of claims. */
export interface Holder {
  session_id: string;
  /** The Claude Code process, as the hook's `CLAUDE_PID` names it. */
  pid: number | null;
  /** When that process started, in ms since the Unix epoch. */
  started: number | null;
  /** The loop the session runs in, or null for an interactive one. */
  loop: string | null;
  goal: string | null;
}

/** One file with uncommitted edits by its session. */
export interface Claim {
  /** The root of the tree the file is in. */
  tree: string;
  /** The file's path from `tree`, with `/` between its parts. */
  path: string;
  /** The subagent that made the first edit, or null for the main conversation. */
  agent_id: string | null;
  /** When the first edit was made, in ms since the Unix epoch. */
  at: number;
  /** The dead session whose claim on the file this one took over. */
  taken_from: Holder | null;
}

/** A session's claim file: its holder's fields, then `touched` and `claims`. */
export interface Session extends Holder {
  /** The time of the session's newest edit. */
  touched: number;
  claims: Claim[];
}

/** One edit the guard is asked about. */
export interface Edit {
  tree: string;
  path: string;
  agent_id: string | null;
  now: number;
}

/** The questions about the machine and the tree that a claim's judgment needs. */
export interface Probe {
  /** When the running process `pid` started, or null when it has exited. */
  started(pid: number): number | null;
  /** Whether `path` in `tree` has no uncommitted change: its working content is `HEAD`'s. */
  clean(tree: string, path: string): boolean;
  /** The paths in `tree` with an uncommitted change, or null when they cannot be read. */
  changed(tree: string): Set<string> | null;
}

/** Runs git in `tree`, and gives its stdout, or null when it fails. */
function git(tree: string, args: string[]): string | null {
  try {
    const ran = Bun.spawnSync(["git", ...args], { cwd: tree, stdin: "ignore", stdout: "pipe", stderr: "ignore" });
    return ran.exitCode === 0 ? ran.stdout.toString() : null;
  } catch {
    return null;
  }
}

/** Asks the OS when a process started, and `git status` which files a tree has changed. */
export const machine: Probe = {
  started,
  clean: (tree, path) =>
    git(tree, ["--literal-pathspecs", "status", "--porcelain", "--untracked-files=all", "--", path]) === "",
  changed(tree) {
    const out = git(tree, ["status", "--porcelain", "-z", "--untracked-files=all"]);
    if (out === null) return null;
    const entries = out.split("\0");
    const paths = new Set<string>();
    // Each entry is `XY path`, and a rename or a copy is followed by its source's path.
    for (let i = 0; i < entries.length; i++) {
      const entry = entries[i] ?? "";
      if (entry.length < 3) continue;
      paths.add(entry.slice(3));
      if (/[RC]/.test(entry.slice(0, 2))) i++;
    }
    return paths;
  },
};

/** The claim folder of the project whose root is `root`. */
export function dir(root: string): string {
  return join(root, ".looper", DIR);
}

/** `sessionId` as a file name: a character outside `[A-Za-z0-9-_]` becomes `_`. */
function fileStem(sessionId: string): string {
  return sessionId.replace(/[^A-Za-z0-9_-]/g, "_");
}

/** The claim file of `sessionId` in `dir`. */
function file(dir: string, sessionId: string): string {
  return join(dir, `${fileStem(sessionId)}.json`);
}

/** `path` as two paths are compared. Windows paths ignore case. */
function key(path: string): string {
  return process.platform === "win32" ? path.toLowerCase() : path;
}

/** Whether `c` is the claim on `path` in `tree`. */
function is(c: Claim, tree: string, path: string): boolean {
  return key(c.tree) === key(tree) && key(c.path) === key(path);
}

function holds(s: Session, tree: string, path: string): boolean {
  return s.claims.some((c) => is(c, tree, path));
}

/** The holder of `s`, with its fields alone. */
function holderOf(s: Holder): Holder {
  return { session_id: s.session_id, pid: s.pid, started: s.started, loop: s.loop, goal: s.goal };
}

/** Whether the session `s` runs, as `probe.started` and the time `now` tell. */
export function live(s: Session, probe: Probe, now: number): boolean {
  if (s.pid !== null && s.started !== null) return probe.started(s.pid) === s.started;
  if (s.loop !== null) return true;
  return now - s.touched < QUIET_MS;
}

/** Reads the session file at `path`. A file that is gone or does not parse is undefined. */
function read(path: string): Session | undefined {
  try {
    const s = readState<Session>(path);
    return s !== undefined && Array.isArray(s.claims) ? s : undefined;
  } catch {
    return undefined;
  }
}

/** Every session file in `dir`, with its path. A file that does not parse is skipped. */
export function list(dir: string): [string, Session][] {
  let names: string[];
  try {
    names = readdirSync(dir);
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") return [];
    throw e;
  }
  const out: [string, Session][] = [];
  for (const name of names) {
    if (!name.endsWith(".json")) continue;
    const path = join(dir, name);
    const s = read(path);
    if (s !== undefined) out.push([path, s]);
  }
  return out;
}

/** Takes the lock beside the session file `path`, runs `body`, and frees the lock. A lock older than `LOCK_STALE_MS` is a crashed writer's, and is removed. */
function locked<T>(path: string, body: () => T): T {
  const lock = path.replace(/\.json$/, ".lock");
  const start = Date.now();
  for (;;) {
    try {
      closeSync(openSync(lock, "wx"));
      break;
    } catch (e) {
      const code = (e as NodeJS.ErrnoException).code;
      if (code !== "EEXIST" && code !== "EPERM" && code !== "EACCES") throw e;
      let age = 0;
      try {
        age = Date.now() - statSync(lock).mtimeMs;
      } catch {}
      if (age > LOCK_STALE_MS) rmSync(lock, { force: true });
      else if (Date.now() - start > LOCK_WAIT_MS) throw new Error(`${lock} is held`);
      else Bun.sleepSync(LOCK_RETRY_MS);
    }
  }
  try {
    return body();
  } finally {
    rmSync(lock, { force: true });
  }
}

/** Writes `s` to `path`, or deletes `path` when `s` holds no claims. */
function store(path: string, s: Session): void {
  if (s.claims.length === 0) rmSync(path, { force: true });
  else writeState(path, s);
}

/** Rewrites the session file at `path` with `change`, under its lock. A file that is gone stays gone. */
function rewrite(path: string, change: (s: Session) => void): void {
  locked(path, () => {
    const s = read(path);
    if (s === undefined) return;
    change(s);
    store(path, s);
  });
}

/** `ms` as a person reads an age: seconds, minutes, hours or days. */
export function age(ms: number): string {
  const s = Math.floor(Math.max(0, ms) / 1000);
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.floor(s / 60)} min`;
  if (s < 86_400) return `${Math.floor(s / 3600)} h`;
  return `${Math.floor(s / 86_400)} d`;
}

/** `path` named as held by `h` for `ms`: its session, loop or `interactive`, goal and the claim's age. */
function held(path: string, h: Holder, ms: number): string {
  const where = h.loop === null ? "interactive" : `loop ${h.loop}`;
  return `${path} has uncommitted edits by session ${h.session_id} (${where}, goal ${h.goal ?? "unknown"}), claimed ${age(ms)} ago`;
}

/** Why `edit` is refused by the claim of the live session `other`. */
function refusal(edit: Edit, other: Session): string {
  const at = other.claims.find((c) => is(c, edit.tree, edit.path))?.at ?? edit.now;
  return `${held(edit.path, other, edit.now - at)}. Work on another file, wait for that session's commit, or ask the user to release its claim.`;
}

/** A memo of each tree's changed paths as keys, so `probe.changed` is asked once per tree. */
type Changed = Map<string, Set<string> | null>;

/** Whether the file of `c` has uncommitted edits, or its tree's changes cannot be read. */
function dirty(c: Claim, changed: Changed, probe: Probe): boolean {
  const tree = key(c.tree);
  if (!changed.has(tree)) {
    const paths = probe.changed(c.tree);
    changed.set(tree, paths === null ? null : new Set([...paths].map(key)));
  }
  const paths = changed.get(tree);
  return paths == null || paths.has(key(c.path));
}

/**
 * Judges `edit` by the session `me`, and records its claim when it passes. A string is a refusal, and
 * null a pass. `goal` names the session's goal when `me` names none, and is asked only when the
 * session's file is first written.
 *
 * Another live session's claim on the file refuses it, unless the file is clean, which releases that
 * claim. A dead session's claim is taken over.
 */
export function claim(dir: string, me: Holder, edit: Edit, probe: Probe, goal: () => string | null): string | null {
  const mine = file(dir, me.session_id);
  const known = me.pid !== null && me.started !== null;
  const own = known ? read(mine) : undefined;
  if (own !== undefined && holds(own, edit.tree, edit.path)) return null;
  let takenFrom: Holder | null = null;
  for (const [path, other] of list(dir)) {
    if (path === mine || !holds(other, edit.tree, edit.path)) continue;
    if (live(other, probe, edit.now)) {
      if (!probe.clean(edit.tree, edit.path)) return refusal(edit, other);
    } else {
      takenFrom = holderOf(other);
    }
    rewrite(path, (s) => {
      s.claims = s.claims.filter((c) => !is(c, edit.tree, edit.path));
    });
  }
  mkdirSync(dir, { recursive: true });
  return locked(mine, () => {
    const s = read(mine) ?? { ...emptySession(me), goal: me.goal ?? goal() };
    if (!holds(s, edit.tree, edit.path)) {
      if (s.claims.length >= MAX_CLAIMS) {
        const changed: Changed = new Map();
        s.claims = s.claims.filter((c) => dirty(c, changed, probe));
      }
      if (s.claims.length >= MAX_CLAIMS) {
        store(mine, s);
        return `this session holds ${MAX_CLAIMS} files with uncommitted edits, the most it may. Commit some of this session's files first.`;
      }
      s.claims.push({ tree: edit.tree, path: edit.path, agent_id: edit.agent_id, at: edit.now, taken_from: takenFrom });
    }
    store(mine, { ...holderOf(me), goal: me.goal ?? s.goal, touched: edit.now, claims: s.claims });
    return null;
  });
}

function emptySession(me: Holder): Session {
  return { ...holderOf(me), touched: 0, claims: [] };
}

/**
 * The first file in `tree` that `staged` picks and another live session than `sessionId` holds with
 * uncommitted edits, named with its holder as an edit's refusal names it. Null when no such file is
 * left.
 *
 * `probe.changed` is asked once, and only when a live session's claim is picked. A picked claim on a
 * clean file is released.
 */
export function conflict(
  dir: string,
  sessionId: string,
  tree: string,
  staged: (path: string) => boolean,
  probe: Probe,
  now: number,
): string | null {
  const mine = file(dir, sessionId);
  const changed: Changed = new Map();
  const picked = (c: Claim) => key(c.tree) === key(tree) && staged(c.path);
  for (const [path, other] of list(dir)) {
    if (path === mine || !other.claims.some(picked) || !live(other, probe, now)) continue;
    const c = other.claims.find((c) => picked(c) && dirty(c, changed, probe));
    if (c !== undefined) return held(c.path, other, now - c.at);
    rewrite(path, (s) => {
      s.claims = s.claims.filter((c) => !picked(c));
    });
  }
  return null;
}

/** One claim as `claims` lists it. */
export interface Listed {
  session_id: string;
  /** The loop the session runs in, or null for an interactive one. */
  loop: string | null;
  goal: string | null;
  /** Whether the session runs, so its claim refuses another session's edit. */
  live: boolean;
  tree: string;
  path: string;
  agent_id: string | null;
  /** When the claim began, in ms since the Unix epoch. */
  at: number;
  /** The id of the dead session whose claim on the file this one took over. */
  taken_from: string | null;
}

/**
 * Every claim in `dir` whose file still has uncommitted edits, by session id and then oldest first.
 * Whether each session is live is judged by `probe.started` and the time `now`.
 *
 * A claim on a clean file is released, asking `probe.changed` once per tree. A tree whose changes
 * cannot be read keeps its claims. A claim the session made after the session file was read is kept.
 */
export function listing(dir: string, probe: Probe, now: number): Listed[] {
  const sessions = list(dir).sort((a, b) => (a[1].session_id < b[1].session_id ? -1 : a[1].session_id > b[1].session_id ? 1 : 0));
  const changed: Changed = new Map();
  const out: Listed[] = [];
  for (const [path, s] of sessions) {
    const isLive = live(s, probe, now);
    const kept = s.claims.filter((c) => dirty(c, changed, probe));
    const clean = s.claims.filter((c) => !kept.includes(c));
    if (clean.length > 0) {
      const gone = new Set(clean.map((c) => `${key(c.tree)}\0${key(c.path)}\0${c.at}`));
      rewrite(path, (f) => {
        f.claims = f.claims.filter((c) => !gone.has(`${key(c.tree)}\0${key(c.path)}\0${c.at}`));
      });
    }
    kept.sort((a, b) => a.at - b.at);
    for (const c of kept) {
      out.push({
        session_id: s.session_id,
        loop: s.loop,
        goal: s.goal,
        live: isLive,
        tree: c.tree,
        path: c.path,
        agent_id: c.agent_id,
        at: c.at,
        taken_from: c.taken_from?.session_id ?? null,
      });
    }
  }
  return out;
}

/** Who released a session's claims, and when: `releaseAsked` writes it to `<session_id>.released`. */
export interface Released extends Holder {
  /** `ui`, `agent` or `cli:<who>`. */
  by: string;
  at: number;
  /** Whether the session ran when its claims were released. */
  live: boolean;
  /** The files whose claims were freed, from their tree's root. */
  paths: string[];
}

function releasedFile(dir: string, sessionId: string): string {
  return join(dir, `${fileStem(sessionId)}.released`);
}

/** The release record of `sessionId` in `dir`, when one was written. */
export function released(dir: string, sessionId: string): Released | undefined {
  try {
    return readState<Released>(releasedFile(dir, sessionId));
  } catch {
    return undefined;
  }
}

/**
 * Frees every claim of `sessionId` in `dir` for `by`, and records who asked. A string is a refusal:
 * the session holds no claims, or `by` is `agent` and the session is live, which only a person may
 * release.
 */
export function releaseAsked(dir: string, sessionId: string, by: string, probe: Probe, now: number): Released | string {
  const s = read(file(dir, sessionId));
  if (s === undefined) return `session ${sessionId} holds no claims in this project.`;
  const isLive = live(s, probe, now);
  if (isLive && by === "agent") {
    return `session ${sessionId} runs, so only the user may release its claims. Ask the user to run \`claims release ${sessionId}\`.`;
  }
  const record: Released = { by, at: now, ...holderOf(s), live: isLive, paths: s.claims.map((c) => c.path) };
  writeState(releasedFile(dir, sessionId), record);
  release(dir, sessionId);
  return record;
}

/** Frees every claim of `sessionId` in `dir`. Says whether the session held any. */
export function release(dir: string, sessionId: string): boolean {
  const path = file(dir, sessionId);
  if (!existsSync(path)) return false;
  return locked(path, () => {
    const was = existsSync(path);
    rmSync(path, { force: true });
    return was;
  });
}

/**
 * Deletes in `dir` each session file with no claims and each untouched for `IDLE_MS` as of `now`, with
 * any lock, tmp file or release record left beside it. Returns how many it deleted.
 */
export function sweep(dir: string, now: number): number {
  let names: string[];
  try {
    names = readdirSync(dir);
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") return 0;
    throw e;
  }
  let deleted = 0;
  for (const name of names) {
    const path = join(dir, name);
    let idle: boolean;
    try {
      idle = now - statSync(path).mtimeMs >= IDLE_MS;
    } catch {
      continue;
    }
    const ext = name.slice(name.lastIndexOf("."));
    const gone =
      ext === ".json" ? idle || read(path)?.claims.length === 0 : ext === ".lock" || ext === ".tmp" || ext === ".released" ? idle : false;
    if (!gone) continue;
    try {
      rmSync(path);
      deleted++;
    } catch {}
  }
  return deleted;
}
