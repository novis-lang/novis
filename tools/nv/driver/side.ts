// A side run's worktree, and the hand-over into it. `bun nv loop --side <slug>` typed anywhere but the side
// goal's own worktree is the launcher: it makes branch `side/<slug>` and its worktree at
// `.agent-tmp/worktrees/side/<slug>` under the main tree when either is missing, installs the tools'
// packages there, copies in the git-ignored files a sweep reads, and starts the worktree's own `bun nv
// loop --side <slug>` inside it. Every turn then roots itself at the worktree, its `.loop/`, its `target/`
// and its `.agent-tmp/`, so a side run and the chain run share no build and no working tree. The one file
// they share is `driver/sweep-lock.ts`'s lock, which lets one of them sweep at a time.
//
// The copies come from the main tree and are never links, and a file already in the worktree is kept.
// The memos are keyed by content, so a verdict the chain run filed answers for the same bytes in the
// worktree, and the first sweep runs what the branch changed rather than the whole floor. The selection
// store is copied the same way (`seedStore`): its footprints name the tree's items and files, never a
// build, so the worktree's first verify selects from what differs from the tree the main store recorded. The database
// fixtures' certificate is a copy of a Docker volume's that `.gitignore` keeps out of git, and a floor
// check that opens the database fails without it. The fuzz corpus directory is made empty rather than
// copied: the floor's `nv playbook --check` needs every path a bullet names to exist, and a fuzz run
// fills it.
//
// What it spends: a second checkout and a second `target/`, and a second WSL target when the goal names
// one, for as long as the side goal lives. `landing` names the steps that delete all of it.

import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { mainRoot } from "../lib/git.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { SIDE_ENV } from "../lib/chain.ts";
import { SelectStore } from "../select/store.ts";
import { enoughDisk } from "./gates.ts";

/** Free space a new worktree needs on top of the run's own floor: one debug and one release build. */
export const WORKTREE_GB = 30;

/** Repo-relative, the git-ignored files a fresh worktree is given from the main tree: the memos, then the fixtures. */
const SEEDS = [".loop/accept-green.json", ".loop/nv-reads.json", ".loop/machine.json", "tests/db/ca.crt"];
/** Repo-relative, the git-ignored directories a fresh worktree is given empty. */
const SEED_DIRS = [".agent-tmp", "fuzz/corpus"];

export function sideBranch(slug: string): string {
  return `side/${slug}`;
}

/** Repo-relative to the main tree, where side goal `slug`'s worktree is. */
export function sideDir(slug: string): string {
  return `.agent-tmp/worktrees/side/${slug}`;
}

function samePath(a: string, b: string): boolean {
  const norm = (p: string) => resolve(p).replace(/[\\/]+$/, "");
  return process.platform === "win32" ? norm(a).toLowerCase() === norm(b).toLowerCase() : norm(a) === norm(b);
}

/** Whether this process runs from side goal `slug`'s own worktree. */
export async function insideWorktree(slug: string): Promise<boolean> {
  return samePath(ROOT, join(await mainRoot(), sideDir(slug)));
}

/**
 * Makes side goal `slug`'s worktree at `dir` when it is missing, from the repository whose main tree is
 * `main`, and readies it for a run. Returns "" when it is ready, or why not.
 */
export async function prepare(slug: string, main: string, dir: string, minFreeGb: number, say: (line: string) => void): Promise<string> {
  const git = async (args: string[]) => {
    const r = await run(["git", ...args], { cwd: main, timeoutMs: 300_000 });
    return r.code === 0 ? "" : `git ${args.join(" ")}: ${(r.stderr || r.stdout).trim().split("\n")[0]}`;
  };
  if (!existsSync(dir)) {
    const disk = enoughDisk(minFreeGb + WORKTREE_GB, main);
    if (disk) return `a new worktree builds everything again, so it needs ${WORKTREE_GB}G on top of the run's own: ${disk}`;
    const branch = sideBranch(slug);
    const known = await run(["git", "rev-parse", "--verify", "--quiet", `refs/heads/${branch}`], { cwd: main, timeoutMs: 60_000 });
    say(known.code === 0 ? `side run: a worktree for the existing branch \`${branch}\` at ${sideDir(slug)}` : `side run: branch \`${branch}\` from \`main\`, in a worktree at ${sideDir(slug)}`);
    const made = known.code === 0 ? await git(["worktree", "add", dir, branch]) : await git(["worktree", "add", "-b", branch, dir, "main"]);
    if (made) return made;
  }
  if (!existsSync(join(dir, "docs", "agent", "goals", "side", `${slug}.md`))) {
    return `branch \`${sideBranch(slug)}\` has no docs/agent/goals/side/${slug}.md -- commit the side goal on \`main\` before its run starts`;
  }
  if (!existsSync(join(dir, "node_modules"))) {
    say("side run: bun install --frozen-lockfile");
    const r = await run(["bun", "install", "--frozen-lockfile"], { cwd: dir, timeoutMs: 600_000 });
    if (r.code !== 0) return `bun install in ${sideDir(slug)}: ${(r.stderr || r.stdout).trim().split("\n").at(-1)}`;
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

const STORE_PATH = ".cache/select.sqlite";

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

/**
 * The launcher: readies side goal `slug`'s worktree and runs the worktree's own `bun nv loop --side <slug>`
 * in it with `args`, the console passed through. Returns that run's exit code, or 2 when the worktree
 * could not be readied.
 */
export async function launchSide(slug: string, args: string[], minFreeGb: number, say: (line: string) => void): Promise<number> {
  const main = await mainRoot();
  const dir = join(main, sideDir(slug));
  const why = await prepare(slug, main, dir, minFreeGb, say);
  if (why) {
    console.error(`nv loop --side: ${why}`);
    return 2;
  }
  say(`side run: handing over to ${sideDir(slug)}`);
  // The run's turns own the Ctrl-C; this waits for them to finish, as `driver/respawn.ts` does.
  const ignore = () => {};
  process.on("SIGINT", ignore);
  try {
    const child = Bun.spawn([process.execPath, join(dir, "tools/nv/main.ts"), "loop", "--side", slug, ...args], {
      cwd: dir,
      env: { ...process.env, [SIDE_ENV]: slug },
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
    });
    return await child.exited;
  } finally {
    process.off("SIGINT", ignore);
  }
}

/**
 * What a person does once side goal `slug` is green, as the run's last words: the steps
 * `docs/agent/goals/README.md` § *Side goals* gives, with this goal's names filled in. `wslTarget` is the
 * side run's WSL target, or null when the goal names none.
 */
export function landing(slug: string, wslTarget: string | null): string {
  const dir = sideDir(slug);
  const branch = sideBranch(slug);
  const wsl = wslTarget ? `, then \`wsl rm -rf ${wslTarget} ${wslTarget}-src\`` : "";
  return [
    `SIDE GOAL GREEN: \`${slug}\` -- every check in its list passes. Land it by hand:`,
    `         1. In ${dir}: \`git rebase main\`, then \`bun nv verify\` and \`bun nv loop --side ${slug} --goal-only\`.`,
    `         2. In ${dir}: delete docs/agent/goals/side/${slug}.md, data/goals/side/${slug}.json and data/goals/side/${slug}.handoff.json, run \`bun nv render\`, and commit.`,
    `         3. In the main tree, while its chain run holds between two sessions (p in its console): \`git merge --ff-only ${branch}\`. If \`main\` has moved, go back to step 1.`,
    `         4. In the main tree: \`git worktree remove ${dir}\` and \`git branch -d ${branch}\`${wsl}.`,
  ].join("\n");
}
