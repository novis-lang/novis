// `bun nv vsix`: rebuilds the VS Code extension's installable file, `editors/vscode/nvs.vsix`, when the
// extension changed since the last one was built.
//
//     bun nv vsix           build it when the extension's tree differs from the one it was last built from
//     bun nv vsix --hook    the same, as looper's `after_retire` hook
//
// **Every goal end that changed the extension rebuilds the `.vsix`.** The user installs the extension
// from that file, and nothing else keeps it current: verify compiles the extension into `out/` and never
// packages it, and CI's copy is an artifact of a run. So looper's `after_retire` hook runs `--hook` after
// `bun nv relink --build --hook`, and a goal ended outside the loop runs `bun nv vsix` once its file is
// deleted.
//
// "Changed" is the git tree `HEAD` holds at `editors/vscode`, compared with the tree the last build
// recorded in `target/nvs.vsix.tree`. So a change made in a side loop's lane reaches the root's `.vsix`
// at the first goal end after it lands, and a goal that did not touch the extension builds nothing.
// Uncommitted extension changes never make a build due, since they are another session's work in
// progress; a build that runs while they exist records no tree, so the next goal end builds again from
// what was committed. The stamp is under `target/`, which the `.vsix` never packs and `bun nv disk`
// may delete; a missing stamp costs one build. In a side loop's lane (`LOOPER_LOOP_KIND` is `side`) the
// hook builds nothing, for the reason `tools/nv/cmd/relink.ts` gives. What it spends: an `npm ci` when
// `node_modules/` is missing and one bundle and package, under a minute, at a goal end that changed the
// extension, and no memory a request holds.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { rel, ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";

export const summary = "rebuild editors/vscode/nvs.vsix when the extension changed since the last build: nv vsix [--hook]";

const EXTENSION = join(ROOT, "editors", "vscode");
const VSIX = join(EXTENSION, "nvs.vsix");
const STAMP = join(ROOT, "target", "nvs.vsix.tree");
const FLAGS = ["--hook"];

/** What decides whether a build is due: the tree `HEAD` holds at the extension, the tree the last build
 *  recorded, and whether the `.vsix` is on disk. */
export interface VsixState {
  tree: string;
  stamp: string | null;
  vsix: boolean;
}

/** Why a build is due, or `null` when the `.vsix` on disk was built from the tree `HEAD` holds. */
export function due(s: VsixState): string | null {
  if (!s.vsix) return "there is no .vsix yet";
  if (s.stamp === null) return "no build recorded the tree it was built from";
  if (s.stamp !== s.tree) return "the extension changed since the last build";
  return null;
}

export async function run(args: string[], env: Record<string, string | undefined> = process.env): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const unknown = args.filter((a) => !FLAGS.includes(a));
  if (unknown.length > 0) {
    console.error(`nv vsix: unknown argument ${unknown[0]}`);
    return 2;
  }
  if (args.includes("--hook") && env.LOOPER_LOOP_KIND === "side") {
    console.log("nv vsix: a side loop's lane builds no .vsix; the root's is rebuilt at the next goal end after this goal lands");
    return 0;
  }
  if (!existsSync(join(EXTENSION, "package.json"))) {
    console.log(`nv vsix: ${rel(EXTENSION)} has no package.json, so there is nothing to build`);
    return 0;
  }
  const tree = await git(["rev-parse", "HEAD:editors/vscode"]);
  const status = await git(["status", "--porcelain", "--", "editors/vscode"]);
  if (tree === null || status === null) {
    console.error("nv vsix: git could not read the extension's tree");
    return 1;
  }
  const why = due({ tree, stamp: existsSync(STAMP) ? readFileSync(STAMP, "utf8").trim() : null, vsix: existsSync(VSIX) });
  if (why === null) {
    console.log(`nv vsix: ${rel(VSIX)} is built from the extension as it stands`);
    return 0;
  }
  console.log(`nv vsix: building ${rel(VSIX)}, because ${why}`);
  if (!existsSync(join(EXTENSION, "node_modules")) && !(await npm(["ci"]))) return 1;
  if (!(await npm(["run", "--silent", "package"])) || !existsSync(VSIX)) return 1;
  if (status === "") {
    mkdirSync(dirname(STAMP), { recursive: true });
    writeFileSync(STAMP, `${tree}\n`);
  }
  console.log(`nv vsix: ${rel(VSIX)} is built; install it with "Install from VSIX..." in the Extensions view`);
  return 0;
}

/** `git args` in the root, trimmed, or `null` when it failed. */
async function git(args: string[]): Promise<string | null> {
  const r = await runProc(["git", ...args]);
  return r.code === 0 ? r.stdout.trim() : null;
}

/** `npm args` in the extension. Prints the tail of its output and returns `false` when it failed. */
async function npm(args: string[]): Promise<boolean> {
  const command = `npm ${args.join(" ")}`;
  const r = await runProc(["npm", ...args], { cwd: EXTENSION, timeoutMs: 15 * 60 * 1000 });
  if (r.code === 0) return true;
  const tail = `${r.stdout}\n${r.stderr}`.trimEnd().split("\n").slice(-15).join("\n");
  console.error(`nv vsix: \`${command}\` failed in ${rel(EXTENSION)} (exit ${r.code}):\n${tail}`);
  return false;
}
