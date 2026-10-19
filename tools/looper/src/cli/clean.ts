// `clean`, as `crates/looper/src/cmd/clean.rs` prints it: deletes the project's old scratch, and the
// worktrees in it whose work is merged.
//
// The scratch directory is `[project] scratch`. Clean never touches what changed within `AGE`, never
// enters a directory named `target`, and deletes nothing inside a git checkout except through git.
//
// - **Worktrees.** Each worktree that `git worktree list` names under the scratch directory is removed
//   with `git worktree remove`, and its branch with `git branch -d`, when all of these hold:
//   - its directory was last modified longer ago than `AGE`;
//   - `git status --porcelain` in it prints nothing;
//   - its `HEAD` is merged into the `HEAD` of the project root.
//
//   Neither git command is forced, so git refuses whatever would lose work, and clean reports it.
//   Clean never removes a side loops' lane, `<scratch>/worktrees/lane-<n>`, whose build output is kept
//   on purpose.
// - **Files.** Each file under the scratch directory last modified longer ago than `AGE` is deleted,
//   and so is each directory that deleting them empties. Clean skips a link, and does not enter a
//   directory that holds a `.git` entry, which is a worktree or a repository.
//
// It prints one line per item it removes or keeps against its rules, then the totals. Each path is
// named from the project root with `/`. It exits 1 when git or the file system refused a removal. With
// `--json` it prints `Cleaned`, from which the text is rendered.
//
// # The bounds it keeps
//
// It holds one directory listing per level of the scratch tree it walks, and one item per removal.

import { type Dirent, existsSync, lstatSync, readdirSync, rmdirSync, statSync, unlinkSync } from "node:fs";
import { basename, dirname, join, relative } from "node:path";
import { loadConfig } from "../goal/config.ts";
import { type Command, Exit, usage } from "./cli.ts";

/** How long ago, in ms, a file or worktree must have last changed before clean removes it. */
export const AGE = 24 * 60 * 60 * 1000;

/** One thing clean removed, or kept against its rules. */
export interface Item {
  kind: "file" | "dir" | "worktree" | "branch";
  /** The path from the root with `/`, or the branch's name. */
  path: string;
  removed: boolean;
  /** Why it was kept, or null. */
  why: string | null;
}

/** What clean did: one item per removal or keep, in order, each path it removed, and the totals. */
export interface Cleaned {
  /** `[project] scratch`. */
  scratch: string;
  exists: boolean;
  items: Item[];
  /** Each file, directory and worktree removed, from the root. */
  changed: string[];
  files: number;
  worktrees: number;
  /** Whether git or the file system refused a removal. */
  refused: boolean;
}

function item(c: Cleaned, kind: Item["kind"], path: string, kept: string | null): void {
  if (kept === null && kind !== "branch") c.changed.push(path);
  c.items.push({ kind, path, removed: kept === null, why: kept });
}

/** The text clean prints: one line per item, then the totals. */
export function cleanedText(c: Cleaned): string {
  if (!c.exists) return `clean: ${c.scratch} does not exist`;
  const lines = c.items.map((i) => {
    const what = i.kind === "branch" ? `the branch ${i.path}` : i.path;
    return i.why === null ? `removed ${what}` : `kept ${what}: ${i.why}`;
  });
  lines.push(`clean: removed ${c.files} files and ${c.worktrees} worktrees`);
  return lines.join("\n");
}

/** `path` from `root` with `/`. */
function rel(root: string, path: string): string {
  return relative(root, path).replaceAll("\\", "/");
}

/** `path` in a form two spellings of one Windows path share: `\`, lower case, no `\\?\`. */
function comparable(path: string): string {
  return path.replace(/^\\\\\?\\/, "").replaceAll("/", "\\").toLowerCase();
}

/** Whether `path` last changed longer ago than `AGE`. */
function old(path: string, now: number): boolean {
  try {
    return now - lstatSync(path).mtimeMs > AGE;
  } catch {
    return false;
  }
}

/** What `git <args>` in `dir` prints, or throws what it printed on stderr when it exits non-zero. */
function git(dir: string, args: string[]): string {
  const ran = Bun.spawnSync(["git", "-C", dir, ...args], { stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  if (ran.exitCode !== 0) throw new Error(ran.stderr.toString().trim());
  return ran.stdout.toString();
}

/** Each worktree of the repository at `root` whose path is under `scratch`, with its branch, or null for a detached one. */
function worktrees(root: string, scratch: string): [string, string | null][] {
  let list: string;
  try {
    list = git(root, ["worktree", "list", "--porcelain"]);
  } catch {
    return [];
  }
  const under = `${comparable(scratch)}\\`;
  const found: [string, string | null][] = [];
  for (const block of list.split(/\r?\n\r?\n/)) {
    let path: string | null = null;
    let branch: string | null = null;
    for (const line of block.split(/\r?\n/)) {
      if (line.startsWith("worktree ")) path = line.slice("worktree ".length);
      else if (line.startsWith("branch refs/heads/")) branch = line.slice("branch refs/heads/".length);
    }
    if (path !== null && comparable(path).startsWith(under)) found.push([path, branch]);
  }
  return found;
}

/** Whether the worktree at `path` is a side loops' lane, `<scratch>/worktrees/lane-<n>`. */
export function lane(path: string): boolean {
  return basename(path).startsWith("lane-") && basename(dirname(path)) === "worktrees";
}

/** Removes the worktree at `path`, and its `branch`, when the rules in the head comment allow it. */
function worktree(root: string, path: string, branch: string | null, c: Cleaned, now: number): void {
  const shown = rel(root, path);
  if (!old(path, now)) return;
  let status: string | null;
  try {
    status = git(path, ["status", "--porcelain"]);
  } catch {
    status = null;
  }
  if (status === null || status.trim() !== "") {
    item(c, "worktree", shown, "it has changes, or git cannot read its status");
    return;
  }
  try {
    const head = git(path, ["rev-parse", "HEAD"]).trim();
    git(root, ["merge-base", "--is-ancestor", head, "HEAD"]);
  } catch {
    item(c, "worktree", shown, "its HEAD is not merged");
    return;
  }
  try {
    git(root, ["worktree", "remove", path]);
  } catch (e) {
    item(c, "worktree", shown, `git refused to remove it: ${(e as Error).message}`);
    c.refused = true;
    return;
  }
  item(c, "worktree", shown, null);
  c.worktrees++;
  if (branch === null) return;
  try {
    git(root, ["branch", "-d", branch]);
    item(c, "branch", branch, null);
  } catch (e) {
    item(c, "branch", branch, `git refused to delete it: ${(e as Error).message}`);
    c.refused = true;
  }
}

/** Deletes each old file under `dir`, and each directory that deleting them empties. */
function sweep(root: string, dir: string, c: Cleaned, now: number): void {
  let entries: Dirent[];
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    const path = join(dir, entry.name);
    const shown = rel(root, path);
    if (entry.isSymbolicLink()) continue;
    if (entry.isDirectory()) {
      if (entry.name === "target" || existsSync(join(path, ".git"))) continue;
      const before = c.files;
      sweep(root, path, c, now);
      if (c.files > before && readdirSync(path).length === 0) {
        try {
          rmdirSync(path);
          item(c, "dir", shown, null);
        } catch {}
      }
    } else if (old(path, now)) {
      try {
        unlinkSync(path);
        item(c, "file", shown, null);
        c.files++;
      } catch (e) {
        item(c, "file", shown, (e as Error).message);
        c.refused = true;
      }
    }
  }
}

/** Clean on the project at `root` with the scratch directory `scratch`, as of `now`. */
export function clean(root: string, scratch: string, now = Date.now()): Cleaned {
  const dir = join(root, scratch);
  let exists = false;
  try {
    exists = statSync(dir).isDirectory();
  } catch {}
  const c: Cleaned = { scratch, exists, items: [], changed: [], files: 0, worktrees: 0, refused: false };
  if (!exists) return c;
  for (const [path, branch] of worktrees(root, dir)) {
    if (!lane(path)) worktree(root, path, branch, c, now);
  }
  sweep(root, dir, c, now);
  return c;
}

export const cleanCommand: Command = {
  name: "clean",
  about: "Deletes scratch older than a day, and the old scratch worktrees that are clean and merged.",
  json: true,
  run: (args) => {
    if (args.length > 0) throw usage(`\`clean\` takes no argument, and got \`${args[0]}\``);
    const root = process.cwd();
    const cleaned = clean(root, loadConfig(root).project.scratch);
    return { code: cleaned.refused ? Exit.refused : Exit.ok, doc: cleaned, text: cleanedText(cleaned) };
  },
};
