// `bun nv side --prepare`: readies a side goal's worktree for a run, and is safe to repeat.
//
//     bun nv side --prepare --hook
//     bun nv side --prepare <slug> [--dir <path>] [--min-free-gb <n>]
//
// looper runs a side loop in a lane, a worktree from its pool at `.agent-tmp/worktrees/lane-<n>` checked
// out on branch `side/<slug>`, or in the loop's own `.agent-tmp/worktrees/side/<slug>` when that exists.
// looper makes the worktree and the branch, and `looper.toml`'s `before_run` hook runs the `--hook` form
// in it once per run. That form reads `LOOPER_LOOP_KIND` and `LOOPER_GOAL`: for a loop that is not a
// side loop it does nothing, and for a side loop it readies the cwd's worktree for that goal. It never
// makes a branch or a worktree, and so checks no disk either.
//
// The `<slug>` form is a person's. It makes branch `side/<slug>` from `main` and its worktree at
// `.agent-tmp/worktrees/side/<slug>` under the main tree, or at `--dir`, when either is missing, after
// checking the disk has room for a second build on top of `--min-free-gb` (`nv disk`'s `MIN_FREE_GB`).
// Then it readies the worktree as the hook does.
//
// Readying (`ready`) refuses a worktree whose branch lacks the goal's file. Then, each only when it is
// missing, it installs the tools' packages, copies in the git-ignored files a sweep reads, and gives the
// worktree a copy of the selection store. The copies come from the main tree, which `mainRoot` finds from
// inside any worktree, and are never links; a file already in the worktree is kept. The selection store
// is copied the same way (`seedStore`): its footprints name the tree's items and files, never a build, so
// the worktree's first verify and first sweep select from what differs from the tree the main store
// recorded, and run what the branch changed rather than the whole floor. The database fixtures'
// certificate is a copy of a Docker volume's that `.gitignore` keeps out of git, and a floor check that
// opens the database fails without it. The fuzz corpus directory is made empty rather than copied: a fuzz
// run needs it to exist, and fills it.
//
// What it spends: a second checkout and a second `target/` per worktree. A lane keeps both after its side
// loop lands, so the next side loop in it builds warm.
//
// It exits 0 when the worktree is ready or the loop is not a side loop, 1 when the worktree could not be
// readied, and 2 on a usage error.

import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { mainRoot } from "../lib/git.ts";
import { run as runProgram } from "../lib/proc.ts";
import { SelectStore } from "../select/store.ts";
import { enoughDisk, MIN_FREE_GB } from "./disk.ts";

export const summary =
  "readies a side goal's worktree, safe to repeat: nv side --prepare --hook (looper's before_run, in the cwd) | nv side --prepare <slug> [--dir <path>] [--min-free-gb <n>]";

const USAGE = "usage: bun nv side --prepare --hook\n       bun nv side --prepare <slug> [--dir <path>] [--min-free-gb <n>]";

/** Free space a new worktree needs on top of the run's own floor: one debug and one release build. */
export const WORKTREE_GB = 30;

/** Repo-relative, the git-ignored files a fresh worktree is given from the main tree: the machine's facts, then the fixtures. */
const SEEDS = [".loop/machine.json", "tests/db/ca.crt"];
/** Repo-relative, the git-ignored directories a fresh worktree is given empty. */
const SEED_DIRS = [".agent-tmp", "fuzz/corpus"];

const STORE_PATH = ".cache/select.sqlite";

const SLUG = /^[a-z0-9][a-z0-9-]*$/;

export function sideBranch(slug: string): string {
  return `side/${slug}`;
}

/** Repo-relative to the main tree, where side goal `slug`'s worktree is. */
export function sideDir(slug: string): string {
  return `.agent-tmp/worktrees/side/${slug}`;
}

/** The disk check and the package install, replaced under test. Each one left out is the real one. */
export interface PrepareSeams {
  /** "" when `root`'s volume has `gb` free, or why not. */
  disk(gb: number, root: string): string;
  /** Installs the tools' packages in `dir`; "" on success, or why not. */
  install(dir: string): Promise<string>;
}

async function bunInstall(dir: string): Promise<string> {
  const r = await runProgram(["bun", "install", "--frozen-lockfile"], { cwd: dir, timeoutMs: 600_000 });
  return r.code === 0 ? "" : (r.stderr || r.stdout).trim().split("\n").at(-1) ?? "";
}

const REAL: PrepareSeams = { disk: enoughDisk, install: bunInstall };

/**
 * Makes side goal `slug`'s worktree at `dir` when it is missing, from the repository whose main tree is
 * `main`, and readies it for a run. Returns "" when it is ready, or why not.
 */
export async function prepare(slug: string, main: string, dir: string, minFreeGb: number, say: (line: string) => void, over: Partial<PrepareSeams> = {}): Promise<string> {
  const s: PrepareSeams = { ...REAL, ...over };
  const git = async (args: string[]) => {
    const r = await runProgram(["git", ...args], { cwd: main, timeoutMs: 300_000 });
    return r.code === 0 ? "" : `git ${args.join(" ")}: ${(r.stderr || r.stdout).trim().split("\n")[0]}`;
  };
  if (!existsSync(dir)) {
    const disk = s.disk(minFreeGb + WORKTREE_GB, main);
    if (disk) return `a new worktree builds everything again, so it needs ${WORKTREE_GB}G on top of the run's own: ${disk}`;
    const branch = sideBranch(slug);
    const known = await runProgram(["git", "rev-parse", "--verify", "--quiet", `refs/heads/${branch}`], { cwd: main, timeoutMs: 60_000 });
    say(known.code === 0 ? `side run: a worktree for the existing branch \`${branch}\` at ${dir}` : `side run: branch \`${branch}\` from \`main\`, in a worktree at ${dir}`);
    const made = known.code === 0 ? await git(["worktree", "add", dir, branch]) : await git(["worktree", "add", "-b", branch, dir, "main"]);
    if (made) return made;
  }
  return ready(slug, main, dir, say, over);
}

/**
 * Readies the existing worktree `dir` for side goal `slug`'s run, from the main tree `main`: the packages,
 * the git-ignored seeds and the selection store, each only when it is missing. Returns "" when it is
 * ready, or why not.
 */
export async function ready(slug: string, main: string, dir: string, say: (line: string) => void, over: Partial<PrepareSeams> = {}): Promise<string> {
  const s: PrepareSeams = { ...REAL, ...over };
  if (!existsSync(join(dir, "docs", "goals", `${slug}.md`))) {
    return `branch \`${sideBranch(slug)}\` has no docs/goals/${slug}.md -- commit the side goal on \`main\` before its run starts`;
  }
  if (!existsSync(join(dir, "node_modules"))) {
    say("side run: bun install --frozen-lockfile");
    const why = await s.install(dir);
    if (why) return `bun install in ${dir}: ${why}`;
  }
  for (const seed of SEEDS) {
    const from = join(main, seed);
    const to = join(dir, seed);
    if (existsSync(to) || !existsSync(from)) continue;
    mkdirSync(dirname(to), { recursive: true });
    copyFileSync(from, to);
  }
  for (const seed of SEED_DIRS) mkdirSync(join(dir, seed), { recursive: true });
  seedStore(join(main, STORE_PATH), join(dir, STORE_PATH));
  return "";
}

/** Gives a new worktree a copy of the selection store, so its first run selects from what the branch
 * changed rather than recording every atom again. The copy is taken through SQLite, so what the main
 * tree's store holds in its write-ahead log is in it too. A worktree that has a store keeps it. */
export function seedStore(from: string, to: string): void {
  if (existsSync(to) || !existsSync(from)) return;
  mkdirSync(dirname(to), { recursive: true });
  const store = new SelectStore(from);
  try {
    store.copyTo(to);
  } finally {
    store.close();
  }
}

/** The top of the worktree that holds `cwd`, or "" when it is not in one. */
async function toplevel(cwd: string): Promise<string> {
  const r = await runProgram(["git", "rev-parse", "--show-toplevel"], { cwd, timeoutMs: 60_000 });
  return r.code === 0 ? resolve(r.stdout.trim()) : "";
}

/** What `side` reads besides its arguments, replaced under test: the main tree, the environment and the cwd. */
export interface SideSeams extends PrepareSeams {
  main: string;
  env: Record<string, string | undefined>;
  cwd: string;
}

/** Parses and runs `--prepare`; returns the exit code. */
export async function side(args: string[], over: Partial<SideSeams> = {}): Promise<number> {
  let slug = "";
  let dir = "";
  let hook = false;
  let minFreeGb: number | null = null;
  let bad = args[0] === "--prepare" ? "" : "the mode is --prepare";
  for (let i = 1; i < args.length && !bad; i++) {
    const a = args[i]!;
    const next = args[i + 1];
    if (a === "--hook") hook = true;
    else if (a === "--dir" && next !== undefined) dir = args[++i]!;
    else if (a === "--min-free-gb" && next !== undefined && Number.isFinite(Number(next)) && Number(next) >= 0) minFreeGb = Number(args[++i]);
    else if (!a.startsWith("-") && slug === "") slug = a;
    else bad = `unexpected argument ${a}`;
  }
  if (!bad && hook && (slug || dir || minFreeGb !== null)) bad = "--hook takes its slug from LOOPER_GOAL and its worktree from the cwd, so no slug, --dir or --min-free-gb";
  if (!bad && !hook && !SLUG.test(slug)) bad = slug ? `${slug} is not a goal slug` : "--prepare needs a slug, or --hook";
  if (bad) {
    console.error(`${USAGE}\nnv side: error: ${bad}`);
    return 2;
  }
  const say = (l: string) => console.log(l);
  if (hook) return hookRun(over, say);
  const main = over.main ?? (await mainRoot());
  const at = dir ? resolve(dir) : join(main, sideDir(slug));
  const why = await prepare(slug, main, at, minFreeGb ?? MIN_FREE_GB, say, over);
  if (why) {
    console.log(`RED ${why}`);
    return 1;
  }
  console.log(`GREEN side goal ${slug}'s worktree is ready at ${at}`);
  return 0;
}

/** `--prepare --hook`: readies the cwd's worktree when looper runs a side loop there, and does nothing otherwise. */
async function hookRun(over: Partial<SideSeams>, say: (line: string) => void): Promise<number> {
  const env = over.env ?? process.env;
  const kind = env.LOOPER_LOOP_KIND ?? "";
  if (kind !== "side") {
    say(`nv side: LOOPER_LOOP_KIND is ${kind ? `\`${kind}\`` : "unset"}, not \`side\`, so there is nothing to prepare`);
    return 0;
  }
  const slug = env.LOOPER_GOAL ?? "";
  if (!SLUG.test(slug)) {
    console.error(`${USAGE}\nnv side: error: LOOPER_GOAL ${slug ? `\`${slug}\` is not a goal slug` : "is unset"}`);
    return 2;
  }
  const cwd = over.cwd ?? process.cwd();
  const at = await toplevel(cwd);
  if (!at) {
    console.log(`RED ${cwd} is not in a git worktree`);
    return 1;
  }
  const main = over.main ?? (await mainRoot(at));
  const why = await ready(slug, main, at, say, over);
  if (why) {
    console.log(`RED ${why}`);
    return 1;
  }
  console.log(`GREEN side goal ${slug}'s worktree is ready at ${at}`);
  return 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return side(args);
}
