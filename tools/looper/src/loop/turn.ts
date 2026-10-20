// The steps of a turn that act on the project's repository after a session: the sweep, the judgment of
// whether the goal is reached, and retirement. It ports `crates/looper-engine/src/turn.rs`.
//
// - `sweep` commits what the session left uncommitted of its own, as
//   `wip(loop): the unfinished slice of session NNNN`. Its own paths are the ones it wrote (see
//   `Served.written`). Every other dirty path is left alone, because a person or another agent may be
//   working in the tree. The commit takes those paths alone (`git commit --only`), so nothing already
//   staged rides along. A failed commit gives those paths back the index entries they had before the
//   sweep's `git add`, so a path staged before stays staged.
// - A sweep while a merge is under way (`mergeUnderWay`) changes nothing and reports `merging`. Git
//   refuses a partial commit there, and the index holds the merge's resolved and unresolved paths.
// - `sweepEnded` sweeps a session that ended with its worker instead of through a turn. Its paths are
//   rebuilt from disk by `writtenBy`: the file-writing tool calls in its `stream.ndjson`, which the
//   adapter parses again, and its `LOOPER_WRITTEN` list. A session that wrote nothing runs no git.
// - `interrupted` writes `.loop/interrupted.json` when the sweep of a session that wrote no status line
//   committed its work, and `wrapped` deletes it. `lastSweep` writes the red checks of an every-session
//   sweep to `.loop/last-sweep.md`, at most `LAST_SWEEP_MAX` bytes, and deletes it once all are green.
//   The run directory `.loop/` is in the shared `.gitignore` block, so git tracks none of it. A side
//   loop in the project root keeps both in its own `.loop/side-<slug>/` (`runDir`).
// - `notReached` says why a goal is not reached: an open stage, a red check, a deferred check or a
//   missing harvest. The machine outranks the claim, so a session's `DONE` counts for nothing here.
// - `retire` retires a reached goal: the harvest gate, then `retireGoal` (checks into the floor as
//   `[project] carry_to_floor` says, off the chain, file deleted, README rendered), then one commit of
//   the goal file and the README alone. Another loop's session may be editing its own goal file in the
//   same tree, so nothing else of the goals directory rides along. A retirement that fails is put back,
//   so the goal stays live. With `[project] harvest = false` there is no harvest gate.
//
// Every git call runs in the project root with no shell, and a message is passed as arguments, so
// nothing the kit writes lands in the project's tracked files.
//
// # The bounds it keeps
//
// - `writtenBy` holds at most one line of `MAX_LINE` bytes and one `WINDOW` of the stream, and keeps at
//   most `WRITTEN_MAX` paths.
// - `.loop/last-sweep.md` holds at most `LAST_SWEEP_MAX` bytes.

import { closeSync, mkdirSync, openSync, readFileSync, readSync, rmSync, unlinkSync, writeFileSync } from "node:fs";
import { isAbsolute, join } from "node:path";
import type { Config } from "../goal/config.ts";
import { retireGoal } from "../goal/edit.ts";
import { type Goal, goalPath, README, Refused } from "../goal/model.ts";
import { harvested, runDir } from "../goal/orient.ts";
import type { Acceptance, Verdict } from "../verify/accept.ts";
import { type AgentAdapter, MAX_LINE, written } from "./adapter.ts";
import { STREAM, WRITTEN_MAX } from "./session.ts";

/** The subject of a sweep commit, before the session's number. */
export const SWEEP_SUBJECT = "wip(loop): the unfinished slice of";

/** The file in a session's folder that its `LOOPER_WRITTEN` list goes to. */
export const WRITTEN_LIST = "written.txt";

/** How much of a stream `writtenBy` reads at a time. */
export const WINDOW = 1 << 20;

/** The run directory's report of a session's unfinished work. */
const INTERRUPTED = "interrupted.json";

/** The run directory's report of the last every-session sweep's red checks. */
const LAST_SWEEP = "last-sweep.md";

/** The most bytes `lastSweep` writes. */
export const LAST_SWEEP_MAX = 64 * 1024;

/** What a sweep did. */
export interface Swept {
  /** The sweep commit, when there was anything of the session's to commit. */
  commit: string | null;
  /** The session's paths it committed, repository-relative. */
  paths: string[];
  /** The dirty paths it left alone, each as `XY path`. */
  left: string[];
  /** Whether a merge was under way, so the sweep changed nothing and its lists are empty. */
  merging: boolean;
}

/** Runs git in `root` with `input` on its stdin, and gives its stdout, or refuses with its stderr. */
export function git(root: string, args: string[], input?: string): string {
  let out: { exitCode: number; stdout: Buffer; stderr: Buffer };
  try {
    const stdin = input === undefined ? "ignore" : Buffer.from(input);
    out = Bun.spawnSync(["git", ...args], { cwd: root, stdin, stdout: "pipe", stderr: "pipe" });
  } catch (e) {
    throw new Refused(`git would not start: ${(e as Error).message}`);
  }
  if (out.exitCode !== 0) throw new Refused(`git ${args.join(" ")}: ${out.stderr.toString().trim()}`);
  return out.stdout.toString();
}

/** A path as the agent named it, relative to `root` with forward slashes, or null when it is outside. */
export function repoPath(root: string, path: string): string | null {
  const norm = (s: string) =>
    (s.startsWith("\\\\?\\") ? s.slice(4) : s).replaceAll("\\", "/").replace(/\/+$/, "");
  const p = norm(path);
  const r = norm(root);
  if (!isAbsolute(path) && !p.startsWith("/")) return p.replace(/^(\.\/)+/, "");
  // Windows paths compare without case.
  const head = p.slice(0, r.length);
  const rest = p.slice(r.length);
  return head.toLowerCase() === r.toLowerCase() && rest.startsWith("/") ? rest.slice(1) : null;
}

/** The dirty paths of the tree, each with its two-letter status. A rename yields both its paths. */
function dirty(root: string): [string, string][] {
  const out = git(root, ["status", "--porcelain=v1", "-z", "--untracked-files=all"]);
  const fields = out.split("\0").filter((f) => f !== "");
  const entries: [string, string][] = [];
  for (let i = 0; i < fields.length; i++) {
    const f = fields[i] as string;
    const code = f.slice(0, 2);
    entries.push([code, f.slice(3)]);
    if ((code.includes("R") || code.includes("C")) && i + 1 < fields.length) {
      entries.push([code, fields[++i] as string]);
    }
  }
  return entries;
}

/** Whether a merge is under way in `root`. */
export function mergeUnderWay(root: string): boolean {
  try {
    git(root, ["rev-parse", "-q", "--verify", "MERGE_HEAD"]);
    return true;
  } catch {
    return false;
  }
}

/**
 * Gives `paths` back the index entries `before` lists, as `git ls-files -s -z` wrote them. Each path is
 * removed from the index first, so a path `before` does not list is untracked again, and a path with
 * unmerged entries gets all its stages back.
 */
function restoreIndex(root: string, paths: string[], before: string): void {
  const hex = git(root, ["rev-parse", "--show-object-format"]).trim() === "sha256" ? 64 : 40;
  const none = "0".repeat(hex);
  git(root, ["update-index", "-z", "--index-info"], paths.map((p) => `0 ${none}\t${p}\0`).join("") + before);
}

/**
 * Commits the session's own uncommitted paths. `index` is the session's number, `why` how it ended.
 * While a merge is under way it runs no other git and reports `merging`.
 */
export function sweep(root: string, index: number, why: string, wrote: Set<string>): Swept {
  if (mergeUnderWay(root)) return { commit: null, paths: [], left: [], merging: true };
  const ours = new Set<string>();
  for (const p of wrote) {
    const path = repoPath(root, p);
    if (path !== null) ours.add(path);
  }
  const swept: Swept = { commit: null, paths: [], left: [], merging: false };
  for (const [code, path] of dirty(root)) {
    if (ours.has(path)) {
      if (!swept.paths.includes(path)) swept.paths.push(path);
    } else {
      swept.left.push(`${code} ${path}`);
    }
  }
  if (swept.paths.length === 0) return swept;
  const subject = `${SWEEP_SUBJECT} session ${String(index).padStart(4, "0")}`;
  const body =
    `The session ended before it committed this (${why}), so looper committed the paths the session itself ` +
    "wrote. It has not been verified.";
  // The paths are file names from `git status`, so no pathspec matches more than its own file.
  const before = git(root, ["--literal-pathspecs", "ls-files", "-s", "-z", "--", ...swept.paths]);
  git(root, ["--literal-pathspecs", "add", "--", ...swept.paths]);
  try {
    git(root, ["--literal-pathspecs", "commit", "-q", "-m", subject, "-m", body, "--only", "--", ...swept.paths]);
  } catch (e) {
    try {
      restoreIndex(root, swept.paths, before);
    } catch (u) {
      throw new Refused(`${(e as Error).message}; putting the index back failed too: ${(u as Error).message}`);
    }
    throw e;
  }
  swept.commit = git(root, ["rev-parse", "HEAD"]).trim();
  return swept;
}

/**
 * The paths the session in the folder `dir` wrote, at most `WRITTEN_MAX`: the file-writing tool calls
 * `adapter` parses out of its `stream.ndjson`, and its `LOOPER_WRITTEN` list beside it. A missing
 * stream or list adds nothing. A line longer than `MAX_LINE` was cut, so it is skipped, and the reader
 * holds at most one line and one window.
 */
export function writtenBy(adapter: AgentAdapter, dir: string): Set<string> {
  const wrote = new Set<string>();
  const note = (path: string) => {
    if (wrote.size < WRITTEN_MAX && path !== "") wrote.add(path);
  };
  const take = (line: Buffer) => {
    for (const e of adapter.parse(line)) {
      const path = written(e);
      if (path !== null) note(path);
    }
  };
  const fd = openOrNull(join(dir, STREAM));
  if (fd !== null) {
    try {
      const window = Buffer.alloc(WINDOW);
      let line: Buffer[] = [];
      let kept = 0;
      let skipping = false;
      for (let n = readSync(fd, window); n > 0; n = readSync(fd, window)) {
        let from = 0;
        while (from < n) {
          const nl = window.indexOf(10, from);
          const to = nl === -1 || nl >= n ? n : nl;
          if (!skipping) {
            line.push(Buffer.from(window.subarray(from, to)));
            kept += to - from;
            if (kept > MAX_LINE) {
              line = [];
              kept = 0;
              skipping = true;
            }
          }
          if (to < n) {
            if (!skipping) take(Buffer.concat(line));
            line = [];
            kept = 0;
            skipping = false;
          }
          from = to + 1;
        }
      }
      if (!skipping && kept > 0) take(Buffer.concat(line));
    } finally {
      closeSync(fd);
    }
  }
  const list = readOrNull(join(dir, WRITTEN_LIST));
  if (list !== null) {
    for (const path of list.split("\n").slice(0, WRITTEN_MAX)) note(path.trim());
  }
  return wrote;
}

/**
 * Sweeps a session that ended with its worker: `writtenBy`, then `sweep` in `root` and `interrupted`,
 * then its `LOOPER_WRITTEN` list is deleted. `index` is the session's number, `why` how it ended.
 */
export function sweepEnded(root: string, index: number, why: string, adapter: AgentAdapter, dir: string): Swept {
  const wrote = writtenBy(adapter, dir);
  const swept = wrote.size === 0 ? { commit: null, paths: [], left: [], merging: false } : sweep(root, index, why, wrote);
  interrupted(root, index, swept, why);
  remove(join(dir, WRITTEN_LIST));
  return swept;
}

/**
 * Writes `.loop/interrupted.json` for the session `index` when its sweep committed something: the
 * session, the commit, the paths and `why` it ended. A sweep that committed nothing writes nothing.
 */
export function interrupted(root: string, index: number, swept: Swept, why: string): void {
  if (swept.commit === null) return;
  const report = { commit: swept.commit, paths: swept.paths, session: index, why };
  const dir = runDir(root);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, INTERRUPTED), `${JSON.stringify(report, null, 2)}\n`);
}

/** Deletes `.loop/interrupted.json`, if it is there. */
export function wrapped(root: string): void {
  remove(join(runDir(root), INTERRUPTED));
}

/**
 * Writes the red `verdicts` to `.loop/last-sweep.md`, one line each with its reason and its log, or
 * deletes the file when none is red. It holds at most `LAST_SWEEP_MAX` bytes, and a last line counts the
 * checks it left out.
 */
export function lastSweep(root: string, verdicts: Verdict[]): void {
  const path = join(runDir(root), LAST_SWEEP);
  const red = verdicts.filter((v) => !v.green);
  if (red.length === 0) {
    remove(path);
    return;
  }
  // Room for the line that counts what was left out.
  const budget = LAST_SWEEP_MAX - 128;
  let text = "";
  let bytes = 0;
  let left = 0;
  for (const v of red) {
    const log = v.log === null ? "" : ` (log: ${v.log})`;
    const line = `- \`${v.id}\` is red: ${v.why}${log}\n`;
    const len = Buffer.byteLength(line);
    if (left === 0 && bytes + len <= budget) {
      text += line;
      bytes += len;
    } else {
      left++;
    }
  }
  if (left > 0) text += `- ${left} more red check(s) left out of this report\n`;
  mkdirSync(runDir(root), { recursive: true });
  writeFileSync(path, text);
}

/** Why `goal` is not reached, one line per reason, or an empty list when it is. */
export function notReached(root: string, cfg: Config, goal: Goal, acc: Acceptance): string[] {
  const why: string[] = [];
  const open = goal.front.stages.filter((s) => s.status !== "done").map((s) => `${s.id}. ${s.title}`);
  if (open.length > 0) why.push(`stage(s) still open in the front matter: ${open.join(", ")}`);
  for (const v of acc.verdicts.filter((v) => !v.green)) why.push(`check \`${v.id}\` is red: ${v.why}`);
  if (acc.deferred.length > 0) why.push(`check(s) not run yet: ${acc.deferred.join(", ")}`);
  const harvest = cfg.project.harvest;
  if (harvest !== null && !harvested(root, cfg, goal.slug)) {
    const section = harvest.section.replace("{slug}", () => goal.slug);
    why.push(`the harvest is missing: ${harvest.file} needs a non-empty \`${section}\` section`);
  }
  return why;
}

/**
 * Retires the reached goal `slug` and commits the goals directory. Gives what it did, in a line.
 *
 * A retirement that fails, such as a commit a `commit-msg` hook refused, is put back. The goal file and
 * the README get their bytes from before, and both are unstaged, so the goal stays live and the tree
 * holds no half of a retirement.
 */
export function retire(root: string, cfg: Config, slug: string): string {
  const harvest = cfg.project.harvest;
  if (harvest !== null && !harvested(root, cfg, slug)) throw new Refused(`${slug} has no harvest in ${harvest.file}`);
  const goalsDir = join(root, cfg.project.goals_dir);
  const dir = cfg.project.goals_dir.replace(/\/+$/, "");
  const kept = [goalPath(goalsDir, slug), join(goalsDir, README)].map((p) => ({ path: p, bytes: readBytes(p) }));
  let said: string;
  try {
    said = retireGoal(goalsDir, slug, cfg.project.carry_to_floor);
  } catch (e) {
    putBack(kept);
    throw e;
  }
  const subject = `chore(goals): ${slug} is reached and retired`;
  const paths = [`${dir}/${slug}.md`, `${dir}/${README}`];
  try {
    git(root, ["add", "-A", "--", ...paths]);
    git(root, ["commit", "-q", "-m", subject, "--only", "--", ...paths]);
  } catch (e) {
    try {
      git(root, ["reset", "-q", "--", ...paths]);
      putBack(kept);
    } catch (u) {
      throw new Refused(`${(e as Error).message}; putting ${slug} back failed too: ${(u as Error).message}`);
    }
    throw e;
  }
  return said;
}

/** Gives each file the bytes it had, or deletes it when it had none, and leaves a file that already matches alone. */
function putBack(kept: { path: string; bytes: Buffer | null }[]): void {
  for (const { path, bytes } of kept) {
    const now = readBytes(path);
    if (bytes !== null && (now === null || !now.equals(bytes))) writeFileSync(path, bytes);
    else if (bytes === null && now !== null) unlinkSync(path);
  }
}

function readBytes(path: string): Buffer | null {
  try {
    return readFileSync(path);
  } catch {
    return null;
  }
}

function readOrNull(path: string): string | null {
  return readBytes(path)?.toString("utf8") ?? null;
}

function openOrNull(path: string): number | null {
  try {
    return openSync(path, "r");
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") return null;
    throw e;
  }
}

function remove(path: string): void {
  rmSync(path, { force: true });
}
