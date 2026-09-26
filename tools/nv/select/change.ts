// What changed between a recorded tree and the working tree.
//
// `git diff --name-status --no-renames <rev>` compares the commit with the files on disk, staged or not,
// and `git ls-files --others --exclude-standard` adds every untracked file that is not ignored. A rename
// is a removal and an addition, so both paths move. A file whose bytes git reports as changed but which
// only moved line endings is still a change: the selection widens rather than guessing.
//
// A recorded tree is a commit and an overlay of the uncommitted paths with their digests then
// (`store.ts`), so the change since it is the diff against the commit, with each overlay path judged by
// its bytes (`sinceOverlay`). `snapshot` takes the tree as it is, for the store to record.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { digest } from "../keys/scan.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import type { Overlay } from "./store.ts";

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

/** Every path that differs between the commits `since` and `until`, sorted by path: a change replayed
 * from history rather than read from the working tree. */
export async function changedBetween(since: string, until: string, root: string = ROOT): Promise<Change[]> {
  const out: Change[] = [];
  const parts = (await git(["diff", "--name-status", "--no-renames", "-z", since, until, "--"], root)).split("\0");
  for (let i = 0; i + 1 < parts.length; i += 2) {
    const code = parts[i]!;
    const path = parts[i + 1]!;
    if (path) out.push({ path, status: code.startsWith("A") ? "added" : code.startsWith("D") ? "deleted" : "modified" });
  }
  return out.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
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

/** The digest of the file at `path` on disk, or null when there is none. */
export function diskDigest(path: string, root: string = ROOT): string | null {
  try {
    return digest(readFileSync(join(root, path)));
  } catch {
    return null;
  }
}

/**
 * The changes since a recorded tree that is a commit and an overlay, from `changes`, the paths that
 * differ from the commit now. A path the overlay does not name was as the commit holds it, so it is
 * changed exactly when git says so. A path it names is changed when its bytes now differ from the ones
 * recorded, and that includes a path put back as the commit holds it. Each status is taken against the
 * recorded tree.
 */
export function sinceOverlay(changes: Change[], overlay: Overlay, root: string = ROOT): Change[] {
  const out = new Map<string, Change>();
  for (const c of changes) if (!(c.path in overlay)) out.set(c.path, c);
  for (const [path, was] of Object.entries(overlay)) {
    const now = diskDigest(path, root);
    if (now === was) continue;
    out.set(path, { path, status: was !== null && now !== null ? "modified" : now !== null ? "added" : "deleted" });
  }
  return [...out.values()].sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}

/** The tree as it is now, as a store records it: `HEAD`, and each path that differs from it with the
 * digest of its bytes, or null for a path `HEAD` has and the disk does not. */
export async function snapshot(root: string = ROOT): Promise<{ commit: string; overlay: Overlay }> {
  const commit = await commitOf("HEAD", root);
  const overlay: Overlay = {};
  for (const c of await changedPaths(commit, root)) overlay[c.path] = c.status === "deleted" ? null : diskDigest(c.path, root);
  return { commit, overlay };
}

/** The bytes of `path` at commit `rev`, or null when it has none there. */
export async function blobAt(rev: string, path: string, root: string = ROOT): Promise<Uint8Array | null> {
  const p = Bun.spawn(["git", "show", `${rev}:${path}`], { cwd: root, stdout: "pipe", stderr: "ignore" });
  const bytes = new Uint8Array(await new Response(p.stdout).arrayBuffer());
  return (await p.exited) === 0 ? bytes : null;
}
