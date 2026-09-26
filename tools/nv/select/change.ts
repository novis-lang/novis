// What changed between a recorded tree and the working tree.
//
// `git diff --name-status --no-renames <rev>` compares the commit with the files on disk, staged or not,
// and `git ls-files --others --exclude-standard` adds every untracked file that is not ignored. A rename
// is a removal and an addition, so both paths move. A file whose bytes git reports as changed but which
// only moved line endings is still a change: the selection widens rather than guessing.

import { existsSync } from "node:fs";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";

export type Status = "added" | "modified" | "deleted";

export interface Change {
  path: string;
  status: Status;
}

async function git(args: string[], root: string): Promise<string> {
  const r = await run(["git", ...args], { cwd: root, timeoutMs: 120_000 });
  if (r.code !== 0) throw new Error(`git ${args.join(" ")}: exit ${r.code}\n${r.stderr.trim()}`);
  return r.stdout;
}

/** The commit `rev` names, in full. */
export async function commitOf(rev: string, root: string = ROOT): Promise<string> {
  return (await git(["rev-parse", "--verify", `${rev}^{commit}`], root)).trim();
}

/** Every path that differs between the commit `since` and the working tree, untracked files included,
 * sorted by path. */
export async function changedPaths(since: string, root: string = ROOT): Promise<Change[]> {
  const out = new Map<string, Status>();
  const diff = await git(["diff", "--name-status", "--no-renames", "-z", since, "--"], root);
  const parts = diff.split("\0");
  for (let i = 0; i + 1 < parts.length; i += 2) {
    const code = parts[i]!;
    const path = parts[i + 1]!;
    if (!path) continue;
    out.set(path, code.startsWith("A") ? "added" : code.startsWith("D") ? "deleted" : "modified");
  }
  const untracked = await git(["ls-files", "--others", "--exclude-standard", "-z"], root);
  for (const path of untracked.split("\0")) if (path) out.set(path, "added");
  return [...out].map(([path, status]) => ({ path, status })).sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}

/** The changes `--paths` names by hand: each path is taken as changed, as added when it is on disk and
 * absent from `since`, as deleted when it is gone. */
export async function namedChanges(paths: string[], since: string, root: string = ROOT): Promise<Change[]> {
  const out: Change[] = [];
  for (const raw of paths) {
    const path = raw.replace(/\\/g, "/").replace(/^\.\//, "");
    const here = existsSync(`${root}/${path}`);
    let there = true;
    try {
      await git(["cat-file", "-e", `${since}:${path}`], root);
    } catch {
      there = false;
    }
    out.push({ path, status: here && !there ? "added" : !here ? "deleted" : "modified" });
  }
  return out;
}

/** The bytes of `path` at commit `rev`, or null when it has none there. */
export async function blobAt(rev: string, path: string, root: string = ROOT): Promise<Uint8Array | null> {
  const p = Bun.spawn(["git", "show", `${rev}:${path}`], { cwd: root, stdout: "pipe", stderr: "ignore" });
  const bytes = new Uint8Array(await new Response(p.stdout).arrayBuffer());
  return (await p.exited) === 0 ? bytes : null;
}
