// The few questions the tools ask git. Each is one `git` run through `proc`.

import { run } from "./proc.ts";
import { ROOT } from "./paths.ts";

async function git(args: string[], cwd: string = ROOT): Promise<string> {
  const r = await run(["git", ...args], { cwd, timeoutMs: 60_000 });
  if (r.code !== 0) throw new Error(`git ${args.join(" ")}: exit ${r.code}\n${r.stderr.trim()}`);
  return r.stdout;
}

/** The commit `HEAD` names. */
export async function head(cwd: string = ROOT): Promise<string> {
  return (await git(["rev-parse", "HEAD"], cwd)).trim();
}

/** The commit that last touched `path`, or "" when none has. */
export async function lastCommit(path: string, cwd: string = ROOT): Promise<string> {
  return (await git(["log", "-1", "--format=%H", "--", path], cwd)).trim();
}

/** Every tracked file under `paths` (all of them when empty), repo-relative. */
export async function tracked(paths: string[] = [], cwd: string = ROOT): Promise<string[]> {
  const out = await git(["ls-files", "-z", "--", ...paths], cwd);
  return out.split("\0").filter((p) => p.length > 0);
}

/** The paths with uncommitted changes, tracked or not, repo-relative. */
export async function dirty(cwd: string = ROOT): Promise<string[]> {
  const out = await git(["status", "--porcelain=v1", "-z", "--untracked-files=all"], cwd);
  const paths: string[] = [];
  const fields = out.split("\0");
  for (let i = 0; i < fields.length; i++) {
    const f = fields[i];
    if (!f || f.length < 4) continue;
    paths.push(f.slice(3));
    // A rename is followed by its source path, which is a field of its own.
    if (f[0] === "R" || f[0] === "C") i++;
  }
  return paths;
}
