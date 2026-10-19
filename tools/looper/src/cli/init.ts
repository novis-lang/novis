// `init <dir> [--name N] [--stack rust,bun]`, as `crates/looper/src/cmd/init.rs` does it: a new project
// with the shared meta layer (decision 0010) and the kit's copy (decision 0027).
//
// It refuses a directory that exists and is not empty, a name that is not lower case letters, digits,
// `-` and `_`, and a stack the kit does not have. Otherwise, in this order:
//
// 1. It finds what the stacks pin: the installed stable rust for `rust`, and the bun that runs the kit
//    for `bun`. A missing rust refuses init before anything is written.
// 2. It creates the directory and runs `git init -b main`.
// 3. It writes every template of the core and the stacks, filled from `Vars`, and then
//    `docs/goals/README.md` from an empty chain. Each file is new, so init never overwrites one.
// 4. It writes the copy of the kit that runs into `tools/looper/`, as sync does, the manifest last.
// 5. It stages everything, marks the commit hook and the `loop` shim executable in the index, and sets
//    `core.hooksPath` to `tools/git-hooks`.
// 6. It commits `chore: initialize the project with looper`, which runs the new hook.
// 7. It registers the project as `project add` does. When the service does not answer, that is exit 3
//    with the project already committed, and `project add` registers it later.
//
// Its `--json` document is `{"message", "changed", "commits", "project"}`: `changed` lists each file
// written, the one commit is `{"sha", "subject", "paths"}`, and `project` is what `project add` prints.

import { existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { defaultConfig, loadConfig } from "../goal/config.ts";
import { emptyChain } from "../goal/model.ts";
import { renderReadme } from "../goal/render.ts";
import { copyPlan } from "../sync/copy.ts";
import { apply, templates } from "../sync/sync.ts";
import { CORE, KIT_DIR, render, varsFromConfig } from "../sync/template.ts";
import { looperHome } from "../jobs/slots.ts";
import { type Command, type Context, Refusal, type Report, usage } from "./cli.ts";
import { addedDoc, configText, configure, projectText, register } from "./project.ts";

/** The message of the commit init makes. */
export const COMMIT = "chore: initialize the project with looper";
/** The files init marks executable: the hook `core.hooksPath` points at, and Git Bash's shim. */
const EXECUTABLE = ["tools/git-hooks/commit-msg", "loop"];

/** Whether `name` may name a new project: every stack's package manager accepts it. */
export function validName(name: string): boolean {
  return /^[a-z0-9][a-z0-9_-]*$/.test(name) && name.length <= 64;
}

/** The version `argv` prints as its word `word`, when `stack` is one of `stacks`; otherwise empty. */
function pinned(stacks: string[], stack: string, argv: string[], word: number): string {
  if (!stacks.includes(stack)) return "";
  let version: string | undefined;
  try {
    const out = Bun.spawnSync(argv, { stdin: "ignore", stdout: "pipe", stderr: "ignore" });
    if (out.exitCode === 0) version = out.stdout.toString().split(/\s+/).filter((w) => w !== "")[word];
  } catch {
    // A program that does not start prints no version.
  }
  if (version === undefined) {
    throw new Refusal(`the ${stack} stack pins the installed version, and \`${argv.join(" ")}\` did not print one. Install ${stack} first.`);
  }
  return version;
}

/** Runs git in `root` and gives its stdout. It refuses on a non-zero exit. */
function git(root: string, args: string[]): string {
  const out = Bun.spawnSync(["git", ...args], { cwd: root, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  if (out.exitCode !== 0) {
    throw new Refusal(`\`git ${args.join(" ")}\` failed in ${root}: ${out.stderr.toString().trim()}`);
  }
  return out.stdout.toString();
}

/** Writes `text` to the new file `rel` under `root`. */
function writeNew(root: string, rel: string, text: string): void {
  const path = join(root, rel);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text, { flag: "wx" });
}

/**
 * What `init` does with `dir`, `name` and `stacks`, the kit's copy taken from the kit at `kit`. It
 * registers the project with the service of `home`, and prints through `ctx` what holds before that.
 */
export async function init(
  dir: string,
  name: string | undefined,
  stacks: string[],
  kit: string,
  home: string | null,
  ctx: Context,
): Promise<Report> {
  const root = resolve(dir);
  if (existsSync(root) && readdirSync(root).length > 0) {
    throw new Refusal(`${root} is not an empty directory. init only creates a project; sync updates one.`);
  }
  const named = name ?? basename(root);
  if (!validName(named)) {
    throw new Refusal(
      `"${named}" cannot name a new project: use lower case letters, digits, '-' and '_', up to 64, starting with a letter or digit. Pass --name.`,
    );
  }
  let all;
  try {
    all = templates(stacks);
  } catch (e) {
    throw new Refusal((e as Error).message);
  }
  const vars = {
    ...varsFromConfig(defaultConfig()),
    name: named,
    stacks,
    rust_version: pinned(stacks, "rust", ["rustup", "run", "stable", "rustc", "--version"], 1),
    bun_version: stacks.includes("bun") ? Bun.version : "",
  };

  mkdirSync(root, { recursive: true });
  git(root, ["init", "-q", "-b", "main"]);
  const paths: string[] = [];
  for (const t of all) {
    const path = render(t.path, vars);
    writeNew(root, path, render(t.text, vars));
    paths.push(path);
  }
  let cfg;
  try {
    cfg = loadConfig(root);
  } catch (e) {
    throw new Refusal(`the looper.toml init wrote: ${(e as Error).message}`);
  }
  const readme = `${cfg.project.goals_dir}/README.md`;
  writeNew(root, readme, renderReadme(emptyChain(), join(root, cfg.project.goals_dir)));
  paths.push(readme);
  const copy = copyPlan(root, kit);
  apply(root, copy);
  paths.push(...copy.map((c) => c.path));

  git(root, ["add", "-A"]);
  git(root, ["update-index", "--chmod=+x", ...EXECUTABLE]);
  git(root, ["config", "core.hooksPath", "tools/git-hooks"]);
  git(root, ["commit", "-q", "-m", COMMIT]);
  const sha = git(root, ["rev-parse", "--short", "HEAD"]).trim();
  const message = `${root}: initialized ${named} with ${[CORE, ...stacks].join(", ")}`;
  const added = configure(root, named);
  if (!ctx.json) ctx.line(`${message}\n${configText(added)}`);
  await register(home, added);
  return {
    doc: { message, changed: paths, commits: [{ sha, subject: COMMIT, paths }], project: addedDoc(added) },
    text: projectText(added),
  };
}

export const initCommand: Command = {
  name: "init",
  about: "Creates a project in an empty directory: the shared rules, the kit's copy and a first commit.",
  json: true,
  run: (args, ctx) => {
    let dir: string | undefined;
    let name: string | undefined;
    const stacks: string[] = [];
    for (let i = 0; i < args.length; i++) {
      const a = args[i] ?? "";
      const [flag, inline] = a.startsWith("--") && a.includes("=") ? [a.slice(0, a.indexOf("=")), a.slice(a.indexOf("=") + 1)] : [a, undefined];
      if (flag === "--name" || flag === "--stack") {
        const value = inline ?? args[++i];
        if (value === undefined) throw usage(`\`${flag}\` needs a value`);
        if (flag === "--name") name = value;
        else stacks.push(...value.split(",").filter((s) => s !== ""));
      } else if (a.startsWith("-") || dir !== undefined) {
        throw usage(`\`init\` takes a directory, \`--name\` and \`--stack\`, and got \`${a}\``);
      } else dir = a;
    }
    if (dir === undefined) throw usage("`init` needs the directory to create the project in");
    return init(dir, name, stacks, KIT_DIR, looperHome(), ctx);
  },
};
