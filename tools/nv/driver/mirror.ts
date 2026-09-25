// The WSL leg's copy of the working tree, on the distro's own disk.
//
// The leg does not run over `/mnt/<drive>/…`. Every file operation on that 9p mount is a round trip to
// Windows, and `nvs run` from the repository root resolves each `[[app]]` root in `nvs.toml` before the
// program starts, so one fixture that finishes in 0.02s from ext4 takes 1.5s from the mount. A copy that
// is out of date is the risk a copy carries, so `syncMirror` runs before every WSL build and brings the
// copy to the working tree exactly as it is on disk: every file git tracks or would add, committed or not,
// and nothing git ignores.
//
// The sync is git's own, and no per-file question crosses the mount. On Windows, `git add -A` into a
// scratch index (`INDEX`, never the real one) turns the working tree into a tree object, which costs one
// stat per file on NTFS. The copy keeps the tree it last took under `refs/nv/tree`. `git pack-objects`
// packs only the objects the new tree has and that one does not, and the pack goes into the distro as one
// stream on `wsl.exe`'s standard input. There `git index-pack` stores it, `git read-tree -u --reset`
// rewrites only the files that differ, and `git clean -ffdx` removes whatever a fixture left behind. An
// unchanged tree sends no pack at all. The tree is excluded by name rather than through a commit, because
// two snapshots share no history and `rev-list` marks an unrelated commit's tree as already sent only when
// it is handed the tree itself.
//
// File modes come from the real index, which seeds the scratch one the first time, so a script is
// executable in the copy when git says it is. Line endings are the same on both sides: `.gitattributes`
// checks everything out as LF.
//
// The one exception to "nothing git ignores" is `CARRIED`: an ignored file the repository's own
// configuration names, which every program run from the root reads before it starts. Those are copied
// over the mount after the clean, when the working tree has them, so the copy fails exactly when the
// checkout would.

import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, isAbsolute, join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";

/** The scratch index the snapshot is taken through, under the repository root. */
export const INDEX = ".agent-tmp/wsl-mirror/index";

/** The ref in the copy that names the tree it holds. */
const REF = "refs/nv/tree";

/** The first sync packs the whole tree; every later one is a few objects. */
const TIMEOUT_MS = 600 * 1000;

/**
 * Ignored files the copy carries anyway. `nvs.toml`'s `[db]` blocks name `tests/db/ca.crt` as their
 * `tls_ca_file`, and it is exported from the database containers rather than committed, so without it
 * every WSL fixture stops at `E0605` before its program is read.
 */
export const CARRIED = ["tests/db/ca.crt"];

/** A Windows path as the distro sees it: `D:\mwl` is `/mnt/d/mwl`. */
export function wslPath(windowsPath: string): string {
  const flat = windowsPath.replaceAll("\\", "/");
  const m = /^([A-Za-z]):(\/.*)?$/.exec(flat);
  if (m === null) return flat;
  return `/mnt/${m[1]!.toLowerCase()}${(m[2] ?? "").replace(/\/+$/, "")}`;
}

/** The `bash` line that copies each `CARRIED` file `root` has into the copy's working directory. */
export function carryLine(root: string): string {
  const lines = CARRIED.filter((f) => existsSync(join(root, f))).map(
    (f) => `mkdir -p ${q(dirname(f))} && cp ${q(`${wslPath(root)}/${f}`)} ${q(f)}`,
  );
  return lines.map((l) => ` && ${l}`).join("");
}

/** The copy of the checkout the distro reaches at `repo`, beside the leg's target directory. */
export function mirrorPath(targetDir: string, repo: string): string {
  return `${targetDir}-src/${repo.replace(/^\/+/, "").replaceAll("/", "-")}`;
}

/** `word` as one `bash` word: as it is when it needs no quoting, else single-quoted. */
export function q(word: string): string {
  if (word !== "" && /^[A-Za-z0-9_./=:,@%+-]+$/.test(word)) return word;
  return `'${word.replaceAll("'", `'\\''`)}'`;
}

function lastLine(text: string): string {
  return text.trim().split("\n").pop() ?? "";
}

async function git(args: string[], root: string, env: Record<string, string> = {}): Promise<{ ok: boolean; out: string; err: string }> {
  const r = await run(["git", ...args], { cwd: root, env, timeoutMs: TIMEOUT_MS });
  return { ok: r.code === 0, out: r.stdout.trim(), err: lastLine(r.stderr) || `exit ${r.code}` };
}

/** The working tree as a git tree id, or the reason there is none. */
async function snapshot(root: string): Promise<{ tree: string } | { fail: string }> {
  const index = join(root, INDEX);
  if (!existsSync(index)) {
    const real = await git(["rev-parse", "--git-path", "index"], root);
    const from = isAbsolute(real.out) ? real.out : join(root, real.out);
    mkdirSync(dirname(index), { recursive: true });
    if (real.ok && existsSync(from)) copyFileSync(from, index);
  }
  const env = { GIT_INDEX_FILE: index };
  const added = await git(["add", "-A"], root, env);
  if (!added.ok) return { fail: `git add -A: ${added.err}` };
  const tree = await git(["write-tree"], root, env);
  return tree.ok ? { tree: tree.out } : { fail: `git write-tree: ${tree.err}` };
}

/** One `bash -c` line inside the distro, with `input` on its standard input. */
async function inWsl(line: string, input?: ReadableStream<Uint8Array>): Promise<{ code: number; out: string; err: string }> {
  const child = Bun.spawn(["wsl.exe", "--exec", "bash", "-c", line], { stdin: input ?? "ignore", stdout: "pipe", stderr: "pipe" });
  const timer = setTimeout(() => child.kill(), TIMEOUT_MS);
  try {
    const [out, err, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
    return { code, out: out.trim(), err: lastLine(err) || `exit ${code}` };
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Brings the copy at `mirror` inside the distro to this checkout's working tree. Returns "" when the copy
 * matches it, else why it could not be brought there.
 */
export async function syncMirror(mirror: string, root: string = ROOT): Promise<string> {
  try {
    return await sync(mirror, root);
  } catch (e) {
    return `cannot start -- ${(e as Error).message ?? e}`;
  }
}

async function sync(mirror: string, root: string): Promise<string> {
  const snap = await snapshot(root);
  if ("fail" in snap) return snap.fail;
  const m = q(mirror);
  const held = await inWsl(`mkdir -p ${m} && cd ${m} && { [ -d .git ] || git init -q; } && { git rev-parse -q --verify ${REF} || true; }`);
  if (held.code !== 0) return `the copy at ${mirror}: ${held.err}`;
  const old = held.out;
  const tidy = `git read-tree -u --reset ${snap.tree} && git clean -qffdx${carryLine(root)}`;
  if (old === snap.tree) {
    const r = await inWsl(`cd ${m} && ${tidy}`);
    return r.code === 0 ? "" : `the copy at ${mirror}: ${r.err}`;
  }
  // The old tree is excluded only while this repository still has it; after a prune, the whole tree goes.
  const known = old !== "" && (await git(["cat-file", "-e", old], root)).ok;
  const revs = known ? `${snap.tree}\n^${old}\n` : `${snap.tree}\n`;
  const pack = Bun.spawn(["git", "pack-objects", "--stdout", "--revs", "--thin", "-q"], {
    cwd: root,
    stdin: new TextEncoder().encode(revs),
    stdout: "pipe",
    stderr: "pipe",
  });
  const apply = `cd ${m} && git index-pack --stdin --fix-thin >/dev/null && git update-ref ${REF} ${snap.tree} && ${tidy} && git gc --auto --quiet`;
  const [r, packed, packErr] = await Promise.all([inWsl(apply, pack.stdout), pack.exited, new Response(pack.stderr).text()]);
  if (packed !== 0) return `git pack-objects: ${lastLine(packErr) || `exit ${packed}`}`;
  return r.code === 0 ? "" : `the copy at ${mirror}: ${r.err}`;
}
