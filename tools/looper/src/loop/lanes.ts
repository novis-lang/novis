// A project's loops as `loop add` makes them, and the lanes its side loops work in (decisions 0016 and
// 0021). It ports `add_loop` and its lane helpers in `crates/looper-server/src/api.rs`, with a file per
// loop where looper has a store row.
//
// # How it works
//
// - **A loop is its folder, `.looper/loops/<name>/`,** and `loop.json` there names what it works
//   (`LoopRecord`): its kind, branch, worktree and goal, and the lane it holds or since when it waits for
//   one. A folder without `loop.json` is a main loop in the project root.
// - **A main loop works the chain in the root,** on the branch checked out there. A project has one.
// - **A side loop works one goal,** is named `side-<slug>`, and gets the branch `side/<slug>`. A new
//   branch starts at `from`, or else at the main loop's branch, or the root's `HEAD` when there is no
//   main loop. A branch that exists is reused. A start point without the goal file is refused, naming
//   what to merge or commit, because its worker could only hold.
// - **A side loop takes the lowest free lane,** `<root>/<scratch>/worktrees/lane-<n>` of `[loop] lanes`.
//   A lane is added detached on first need and switched to the loop's branch. A lane that is not this
//   repository's worktree, or that has uncommitted files, is refused, so it never overwrites work.
// - **With no lane free, or while an earlier side loop waits, it is added waiting** with no worktree.
//   Its `waiting` names why, and it cannot start until it has a lane.
// - **A side loop whose own worktree, `<root>/<scratch>/worktrees/side/<slug>`, exists keeps it** and
//   takes no lane. That worktree must be this repository's, on `side/<slug>`, holding the goal file.
// - **`handBack` frees a loop's lane** once `git status --porcelain` in it is empty. The lane switches to
//   a detached `HEAD` at the main branch, and goes to the side loop that has waited longest.
// - **A landed side loop keeps its folder, with `landed` in `loop.json`** (`markLanded`), and leaves the
//   list `status` shows. A new side loop on the same goal renames that folder `<name>@landed-<when>`
//   first, which no loop's name can be.
// - **One named mutex per project serialises every add and hand-back,** so two loops never take one
//   lane. A mutex rather than a lock file, because the OS frees it when a killed process dies.
//
// Each loop's record is a few hundred bytes, so the loops folder grows with the loops a project has.

import { existsSync, mkdirSync, readdirSync, realpathSync, renameSync } from "node:fs";
import { join, relative } from "node:path";
import { loadConfig } from "../goal/config.ts";
import { goalPath, Refused } from "../goal/model.ts";
import { type Mutex, openMutex } from "../os/index.ts";
import { readState, writeState } from "../store/state.ts";
import { git } from "./turn.ts";

/** The record of what a loop works, in its folder. */
export const LOOP_FILE = "loop.json";
/** How often a wait on the mutex tries it again, in ms. */
const LOCK_EVERY_MS = 5;

/** What `loop.json` holds. */
export interface LoopRecord {
  kind: "main" | "side";
  branch: string;
  /** The loop's worktree, empty while a side loop waits for a lane. */
  worktree: string;
  /** A side loop's goal, null for a main loop. */
  goal: string | null;
  /** The lane a side loop holds, null when it holds none. */
  lane: number | null;
  /** Since when a side loop waits for a lane, in ms since the Unix epoch, or null. */
  laneWait: number | null;
  /** When a side loop landed, in ms since the Unix epoch. Left out until it lands. */
  landed?: number;
}

/** Why a loop waits for the user, and since when. */
export interface Waiting {
  why: string;
  since: number;
}

/** A loop of the project, by name, with its record. */
export interface Named extends LoopRecord {
  name: string;
}

/** What `addLoop` is asked to add: a main loop named `name`, or a side loop on goal `side`. */
export interface NewLoop {
  name?: string;
  side?: string;
  from?: string;
}

/** The folder that holds the loops of the project at `root`. */
export function loopsDir(root: string): string {
  return join(root, ".looper", "loops");
}

/** The record of the loop whose folder is `dir`, as a main loop in `root` when it has none. */
export function loopRecord(root: string, dir: string): LoopRecord {
  return readState<LoopRecord>(join(dir, LOOP_FILE)) ?? { kind: "main", branch: "", worktree: root, goal: null, lane: null, laneWait: null };
}

/** Every loop of the project at `root`, sorted by name. */
export function loops(root: string): Named[] {
  const dir = loopsDir(root);
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .sort()
    .map((name) => ({ name, ...loopRecord(root, join(dir, name)) }));
}

/** Why loop `l` waits for a lane in a project of `lanes` lanes, or null when it does not. */
export function laneWaiting(l: LoopRecord, lanes: number): Waiting | null {
  return l.laneWait === null ? null : { why: `no free lane (lanes = ${lanes})`, since: l.laneWait };
}

function mutexName(root: string): string {
  const key = Bun.hash(join(root, ".looper").replaceAll("/", "\\").toLowerCase()).toString(16);
  return `Local\\looper-kit-lanes-${key}`;
}

/** Runs `body` under the project's lane mutex, waiting while another process holds it. The landing's lock takes it too. */
export async function locked<T>(root: string, body: () => T): Promise<T> {
  const m: Mutex = openMutex(mutexName(root));
  while (!m.tryTake()) await Bun.sleep(LOCK_EVERY_MS);
  try {
    return body();
  } finally {
    m.close();
  }
}

/** Adds the loop `want` asks for to the project at `root`, idle or waiting for a lane, and gives it. */
export function addLoop(root: string, want: NewLoop): Promise<Named> {
  return locked(root, () => (want.side === undefined ? addMain(root, want) : addSide(root, want.side, want.from, want.name)));
}

function write(root: string, name: string, record: LoopRecord): Named {
  const dir = join(loopsDir(root), name);
  mkdirSync(dir, { recursive: true });
  const { kind, branch, worktree, goal, lane, laneWait, landed } = record;
  writeState(join(dir, LOOP_FILE), { kind, branch, worktree, goal, lane, laneWait, landed });
  return { name, kind, branch, worktree, goal, lane, laneWait, landed };
}

/** Marks side loop `name` of the project at `root` landed, so it leaves the list and a new side loop may take its name. */
export function markLanded(root: string, name: string): Promise<void> {
  return locked(root, () => {
    const l = loops(root).find((x) => x.name === name);
    if (l !== undefined && l.landed === undefined) write(root, name, { ...l, landed: Date.now() });
  });
}

function addMain(root: string, want: NewLoop): Named {
  if (want.from !== undefined) throw new Refused("a main loop works the root's branch, so it takes no `--from`");
  const name = want.name ?? "main";
  const all = loops(root);
  if (all.some((l) => l.name === name)) throw new Refused(`this project has the loop ${name} already`);
  const main = all.find((l) => l.kind === "main");
  if (main !== undefined) throw new Refused(`this project has a main loop already: ${main.name}`);
  const branch = git(root, ["rev-parse", "--abbrev-ref", "HEAD"]).trim();
  return write(root, name, { kind: "main", branch, worktree: root, goal: null, lane: null, laneWait: null });
}

function addSide(root: string, slug: string, from: string | undefined, asked: string | undefined): Named {
  const cfg = loadConfig(root);
  const goals = join(root, cfg.project.goals_dir);
  if (!/^[a-z0-9][a-z0-9-]*$/.test(slug) || !existsSync(goalPath(goals, slug))) throw new Refused(`this project has no goal ${slug}`);
  const name = `side-${slug}`;
  if (asked !== undefined && asked !== name) throw new Refused(`a side loop on ${slug} is named ${name}, not ${asked}`);
  let all = loops(root);
  const old = all.find((l) => l.name === name);
  if (old?.landed !== undefined) {
    // A landed loop keeps its folder under a name no loop takes, which frees its own.
    renameSync(join(loopsDir(root), name), join(loopsDir(root), `${name}@landed-${old.landed}`));
    all = loops(root);
  } else if (old !== undefined) throw new Refused(`this project has the loop ${name} already`);
  const worktrees = join(root, cfg.project.scratch, "worktrees");
  const own = join(worktrees, "side", slug);
  const branch = `side/${slug}`;
  // The goal file as git names it from the root, which the side loop's start point must hold.
  const file = relative(root, goalPath(goals, slug)).replaceAll("\\", "/");
  const side = (worktree: string, lane: number | null, laneWait: number | null) =>
    write(root, name, { kind: "side", branch, worktree, goal: slug, lane, laneWait });
  if (existsSync(own)) {
    sideWorktree(root, own, branch, file);
    return side(own, null, null);
  }
  sideBranch(root, branch, file, from ?? all.find((l) => l.kind === "main")?.branch ?? "HEAD");
  const n = freeLane(all, cfg.loop.lanes);
  if (n === null) return side("", null, Date.now());
  const path = join(worktrees, `lane-${n}`);
  readyLane(root, path, branch, n);
  return side(path, n, null);
}

/** The lowest of `lanes` lanes no loop of `all` holds, or null while a side loop waits already, because it takes the next lane to free first. */
function freeLane(all: LoopRecord[], lanes: number): number | null {
  if (all.some((l) => l.laneWait !== null)) return null;
  for (let n = 1; n <= lanes; n++) if (!all.some((l) => l.lane === n)) return n;
  return null;
}

/** Readies `branch` from the root for a side loop whose goal file is `file`: it makes the branch at `from` when it does not exist. A start point without `file` is refused. */
function sideBranch(root: string, branch: string, file: string, from: string): void {
  const has = succeeds(root, ["rev-parse", "--verify", "--quiet", `refs/heads/${branch}`]);
  if (!succeeds(root, ["cat-file", "-e", `${has ? branch : from}:${file}`])) {
    throw new Refused(
      has
        ? `the branch ${branch} has no ${file}: merge the branch that holds it into ${branch}, or commit it there`
        : `${from} has no ${file}: commit it there, or start the side loop \`--from\` a branch that holds it`,
    );
  }
  if (!has) git(root, ["branch", branch, from]);
}

/** Readies lane `n`, the worktree at `path`, and switches it to `branch`. A lane is added detached on first need. */
function readyLane(root: string, path: string, branch: string, n: number): void {
  if (existsSync(path)) {
    if (!namesWorktree(root, path, null)) throw new Refused(`${path} exists already, and is not this repository's worktree`);
    if (git(path, ["status", "--porcelain"]).trim() !== "") throw new Refused(`lane-${n} has uncommitted files`);
  } else {
    git(root, ["worktree", "add", "--detach", path, branch]);
  }
  git(path, ["switch", branch]);
}

/** Checks a side loop's own worktree at `path`, which exists, for reuse: it must be this repository's worktree on `branch`, and hold the goal file `file`. */
function sideWorktree(root: string, path: string, branch: string, file: string): void {
  if (!namesWorktree(root, path, branch)) throw new Refused(`${path} exists already, and is not this repository's worktree on ${branch}`);
  if (!existsSync(join(path, file))) {
    throw new Refused(`the worktree ${path} has no ${file}: merge the branch that holds it into ${branch}, or commit it there`);
  }
}

/** Whether `git worktree list` in `root` names `path` as a worktree, checked out on `branch` when one is named. */
function namesWorktree(root: string, path: string, branch: string | null): boolean {
  const want = canonical(path);
  if (want === null) return false;
  return git(root, ["worktree", "list", "--porcelain"])
    .replaceAll("\r\n", "\n")
    .split("\n\n")
    .some((entry) => {
      const field = (key: string) => entry.split("\n").find((l) => l.startsWith(key))?.slice(key.length);
      const at = field("worktree ");
      return at !== undefined && canonical(at) === want && (branch === null || field("branch ") === `refs/heads/${branch}`);
    });
}

/** `path` resolved and in one case, since Windows paths compare without it, or null when it does not exist. */
function canonical(path: string): string | null {
  try {
    return realpathSync.native(path).replaceAll("/", "\\").toLowerCase();
  } catch {
    return null;
  }
}

function succeeds(root: string, args: string[]): boolean {
  return Bun.spawnSync(["git", ...args], { cwd: root, stdin: "ignore", stdout: "ignore", stderr: "ignore" }).exitCode === 0;
}

/**
 * Hands back the lane of loop `name` once it is clean, switched to a detached `HEAD` at `main`, and gives
 * it to the side loop that has waited longest. Gives that loop's name, or null when none waited. A lane
 * with uncommitted files stays with its loop, and is refused with why.
 */
export function handBack(root: string, name: string, main: string): Promise<string | null> {
  return locked(root, () => {
    const all = loops(root);
    const l = all.find((x) => x.name === name);
    if (l === undefined || l.lane === null) return null;
    const lanes = loadConfig(root).loop.lanes;
    if (git(l.worktree, ["status", "--porcelain"]).trim() !== "") throw new Refused(`lane-${l.lane} has uncommitted files`);
    git(l.worktree, ["switch", "--detach", main]);
    const path = l.worktree;
    const n = l.lane;
    write(root, name, { ...l, worktree: "", lane: null });
    if (n > lanes) {
      // A lane past the pool size is deleted once it is free and clean, never with `--force`.
      git(root, ["worktree", "remove", path]);
      return null;
    }
    const next = all.filter((x) => x.laneWait !== null).sort((a, b) => (a.laneWait as number) - (b.laneWait as number))[0];
    if (next === undefined) return null;
    readyLane(root, path, next.branch, n);
    write(root, next.name, { ...next, worktree: path, lane: n, laneWait: null });
    return next.name;
  });
}
