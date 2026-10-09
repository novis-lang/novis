// A side run's hand-over into its worktree. `bun nv loop --side <slug>` typed anywhere but the side goal's
// own worktree is the launcher: it readies the worktree with `cmd/side.ts`'s `prepare`, the code `bun nv
// side --prepare` runs, and starts the worktree's own `bun nv loop --side <slug>` inside it. Every turn
// then roots itself at the worktree, its `.loop/`, its `target/` and its `.agent-tmp/`, so a side run and
// the chain run share no build and no working tree. The one file they share is `accept/sweep-lock.ts`'s
// lock, which lets one of them sweep at a time.
//
// What it spends: a second WSL target when the goal names one, on top of the worktree `cmd/side.ts`
// spends, for as long as the side goal lives. `landing` names the steps that delete all of it.

import { join, resolve } from "node:path";
import { prepare, sideBranch, sideDir } from "../cmd/side.ts";
import { mainRoot } from "../lib/git.ts";
import { ROOT } from "../lib/paths.ts";
import { SIDE_ENV } from "../lib/chain.ts";

function samePath(a: string, b: string): boolean {
  const norm = (p: string) => resolve(p).replace(/[\\/]+$/, "");
  return process.platform === "win32" ? norm(a).toLowerCase() === norm(b).toLowerCase() : norm(a) === norm(b);
}

/** Whether this process runs from side goal `slug`'s own worktree. */
export async function insideWorktree(slug: string): Promise<boolean> {
  return samePath(ROOT, join(await mainRoot(), sideDir(slug)));
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
