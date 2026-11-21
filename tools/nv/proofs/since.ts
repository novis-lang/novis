// The features a stretch of work reaches: the scope `bun nv proofs --since <rev>` and `--goal <slug>`
// judge. `--owed --goal <slug>` is a goal's worklist of feature proofs, and `--gate --goal <slug>` is
// its check, so no goal lists its features by hand. A hand list is written before the code exists, and
// it missed every member of the value classes a goal added.
//
// A goal's work is every change since the commit that added its file, `docs/goals/<slug>.md`, together
// with the uncommitted and untracked files of the tree. A feature is reached when that work changed a
// file it lives in:
//
//   - its anchor file, where the roster found its code;
//   - a file under its own proof paths: its example directory, its attack directory, its bench, and its
//     group's directory under `data/proofs/skip/`; a language feature's chapter source too;
//   - for a feature with no anchor, a changed source file that declares its class: a Rust file with the
//     class's name as a string literal, or a Novis file that declares the class in its namespace. The
//     members of a value class such as `Core\Test\RunResult` have no anchor, and this is how a new
//     one is reached.
//
// The work since that commit also holds other goals' commits when the goal was written before it went
// live. Each of those goals closed with its own reach green, so the extra features are read and owe
// nothing. A feature a running side loop is still building is reached only through a file both goals
// changed.

import { existsSync } from "node:fs";
import { dirname } from "node:path";
import { abs } from "../lib/paths.ts";
import { benchFile, examplesDir, hostileDir, implFile, read, type Entry } from "./roster.ts";

export const SKIP_DIR = "data/proofs/skip";
export const GOALS_DIR = "docs/goals";

/** One git command's standard output, or the error that names it. */
function git(args: string[]): string {
  const r = Bun.spawnSync(["git", ...args], { cwd: abs("."), stdout: "pipe", stderr: "pipe" });
  if (r.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${r.stderr.toString().trim() || `exit ${r.exitCode}`}`);
  return r.stdout.toString();
}

const lines = (text: string) => text.split("\n").map((l) => l.trim()).filter((l) => l !== "");

/** The commit `--goal <slug>` measures from: the newest one that added `docs/goals/<slug>.md`. A goal
 * file not committed yet measures from `HEAD`, and a slug with no file in the tree or its history is an
 * error. */
export function goalBase(slug: string): string {
  const file = `${GOALS_DIR}/${slug}.md`;
  const added = lines(git(["log", "--diff-filter=A", "--format=%H", "-1", "--", file]))[0];
  if (added) return added;
  if (existsSync(abs(file))) return "HEAD";
  throw new Error(`no goal ${slug}: ${file} is not in the tree or its history`);
}

/** Every repo-relative path the work since `rev` changed: committed, staged, unstaged and untracked. */
export function changedSince(rev: string): string[] {
  git(["rev-parse", "--verify", "--quiet", `${rev}^{commit}`]);
  const tracked = lines(git(["diff", "--name-only", "--no-renames", rev, "--"]));
  const untracked = lines(git(["ls-files", "--others", "--exclude-standard"]));
  return [...new Set([...tracked, ...untracked])].sort();
}

/** The class a feature with no anchor is declared in: a member's group, or the type itself, without the
 * `::` of a class-scoped enum. */
export const classOf = (e: Entry) => (e.kind === "member" ? e.group : e.id).split("::")[0]!;

/** Whether a Rust file's text names `klass` as a string literal, raw or escaped. */
export function rustNames(text: string, klass: string): boolean {
  return text.includes(`"${klass}"`) || text.includes(`"${klass.replaceAll("\\", "\\\\")}"`);
}

/** Whether a Novis file's text declares `klass`: its namespace line, and a class, enum or interface
 * of the short name. */
export function novisDeclares(text: string, klass: string): boolean {
  const cut = klass.lastIndexOf("\\");
  if (cut < 0) return false;
  const ns = klass.slice(0, cut).replaceAll("\\", "\\\\");
  const short = klass.slice(cut + 1);
  return new RegExp(`^\\s*namespace\\s+${ns}\\s*;`, "m").test(text) && new RegExp(`\\b(?:class|enum|interface)\\s+${short}\\b`).test(text);
}

/** The entries the changed paths reach, in roster order. `text` reads a repo-relative file. */
export function reached(entries: Entry[], changed: string[], text: (path: string) => string = read): Entry[] {
  const files = new Set(changed);
  const under = (dir: string) => changed.some((p) => p === dir || p.startsWith(`${dir}/`));
  const sources = changed.filter((p) => p.endsWith(".rs") || p.endsWith(".nvs")).map((p) => [p, text(p)] as const);
  const memo = new Map<string, boolean>();
  const declared = (klass: string) => {
    let hit = memo.get(klass);
    if (hit === undefined) {
      hit = sources.some(([p, t]) => (p.endsWith(".rs") ? rustNames(t, klass) : novisDeclares(t, klass)));
      memo.set(klass, hit);
    }
    return hit;
  };
  return entries.filter(
    (e) =>
      (e.anchor !== "" && files.has(implFile(e))) ||
      under(examplesDir(e)) ||
      under(hostileDir(e)) ||
      files.has(benchFile(e)) ||
      under(`${SKIP_DIR}/${dirname(e.path)}`) ||
      (e.kind === "lang" && files.has(`${dirname(examplesDir(e))}/chapter.md`)) ||
      (e.anchor === "" && declared(classOf(e))),
  );
}
