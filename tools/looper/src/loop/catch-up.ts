// The catch-up: what a side loop does once its goal is retired, before it lands (decision 0016). It merges
// the main branch into `side/<slug>`, resolves conflicts only in generated files, runs verify, and commits
// the merge once it is green. It ports `Driver::catch_up` and `merged_chain` in
// `crates/looper-engine/src/driver.rs`. The driver calls it and turns its `Caught` into an outcome.
//
// # The steps
//
// 1. A merge already under way, such as one a killed worker left before its commit, is finished: it goes
//    on to the verify in step 2. One with conflicts holds, and is never aborted. Otherwise, a side branch
//    with uncommitted tracked changes holds, because aborting a merge could lose them.
// 2. It runs `git merge --no-ff --no-commit <main>` in the side worktree, then `Side.verify` there.
// 3. A conflict only in generated files is resolved with no session. They are the goals README and the
//    paths `[merge] generated` declares. Each declared path takes the main branch's side. The README's
//    chain takes main's, with the side branch's own changes since the merge base laid over it
//    (`mergedChain`), so the side's retired slug stays off it and its floor checks stay on it. Then the
//    README is rendered, each `[merge] render` command runs in order, and the changed tracked files and
//    the new declared ones are staged. The catch-up goes on to the verify.
// 4. Any other conflict, a render command that fails, or a red verify aborts the merge and asks for a
//    landing session (`landing`), handed the conflicting files, or the red command and its log. A merge
//    git refuses for another reason holds with git's reason.
// 5. A green verify commits the merge, and the catch-up says `land`.
//
// It writes no file git tracks except the generated files it resolves and the merge commit.

import { join } from "node:path";
import type { Config } from "../goal/config.ts";
import { mergeDeclares } from "../goal/config.ts";
import { writeFile } from "../goal/edit.ts";
import { type Chain, type Check, emptyCheck, loadChain, parseChain, README } from "../goal/model.ts";
import { renderReadme } from "../goal/render.ts";
import { type Progress, type Project, runCheck, runReported, type Units } from "../verify/accept.ts";
import { git, mergeUnderWay } from "./turn.ts";

/** How long the catch-up's verify may run, in seconds. */
export const CATCH_UP_VERIFY_S = 7200;

/** What a side loop's driver knows beyond a main loop's: its goal, and where it lands. */
export interface Side {
  /** The one goal the loop works. */
  goal: string;
  /** The project's main checkout, where the branch is merged. */
  mainRoot: string;
  /** The branch the side branch catches up with and is merged into. */
  mainBranch: string;
  /** The argv of verify, which the catch-up runs in the side worktree. Empty runs none. */
  verify: string[];
}

/** What a catch-up runs with. */
export interface Merging {
  /** The side worktree. */
  root: string;
  cfg: Config;
  side: Side;
  /** The retired goal. */
  slug: string;
  checkLogs: string;
  units: Units;
  project: Project;
  progress: Progress;
}

/** How a catch-up ended: the branch is ready to land, a landing session must merge it, or a person must. */
export type Caught = { kind: "land"; said: string } | { kind: "landing"; verdict: string } | { kind: "hold"; why: string };

/** Whether `git <args>` exits 0 in `root`. */
function gitOk(root: string, args: string[]): boolean {
  try {
    git(root, args);
    return true;
  } catch {
    return false;
  }
}

/** The paths the merge under way in `root` leaves conflicted. */
function conflicted(root: string): string[] {
  return git(root, ["diff", "--name-only", "--diff-filter=U"])
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l !== "");
}

/** Aborts the merge in `root`, when one is under way. */
export function abortMerge(root: string): void {
  if (mergeUnderWay(root)) git(root, ["merge", "--abort"]);
}

/** The goals README's root-relative path, as git names it. */
function readmePath(cfg: Config): string {
  return `${cfg.project.goals_dir.replaceAll("\\", "/").replace(/\/+$/, "")}/${README}`;
}

/** The catch-up of the retired side goal `m.slug`, as the module head's steps say. */
export async function catchUp(m: Merging): Promise<Caught> {
  const { root, slug } = m;
  const main = m.side.mainBranch;
  const into = `merging ${main} into side/${slug}`;
  // A merge under way is one a killed worker left between its merge and its commit, or one a landing
  // session left. It is finished rather than aborted, so nothing in it is lost.
  if (mergeUnderWay(root)) {
    const left = conflicted(root);
    if (left.length > 0) {
      return {
        kind: "hold",
        why: `catch-up: side/${slug} has a merge under way with conflicts in these files:\n${left.join("\n")}\nIn this worktree, resolve them and commit.`,
      };
    }
    return verifyAndCommit(m, into);
  }
  const dirty = git(root, ["status", "--porcelain", "--untracked-files=no"]).trimEnd();
  if (dirty !== "") {
    return { kind: "hold", why: `catch-up: side/${slug} has uncommitted changes, so looper does not start ${into}:\n${dirty}` };
  }
  const merged = Bun.spawnSync(["git", "merge", "--no-ff", "--no-commit", main], { cwd: root, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  if (merged.exitCode !== 0) {
    const files = conflicted(root);
    if (files.length === 0) {
      abortMerge(root);
      const said = [merged.stderr.toString().trim(), merged.stdout.toString().trim()].find((s) => s !== "") ?? "";
      return { kind: "hold", why: `catch-up: ${into} failed: ${said}` };
    }
    const readme = readmePath(m.cfg);
    if (files.every((p) => p === readme || mergeDeclares(m.cfg.merge, p))) {
      const why = await renderGenerated(m, files, readme, into);
      if (why === null) return verifyAndCommit(m, into);
      abortMerge(root);
      return { kind: "landing", verdict: why };
    }
    abortMerge(root);
    return {
      kind: "landing",
      verdict: `landing: ${into} conflicts in these files:\n${files.map((f) => `- ${f}`).join("\n")}\nIn this worktree, run \`git merge --no-ff ${main}\` and resolve them.`,
    };
  }
  return verifyAndCommit(m, into);
}

/** The end of a catch-up whose merge has no conflicts: runs verify, then commits the merge once it is green, or aborts it once it is red. */
async function verifyAndCommit(m: Merging, into: string): Promise<Caught> {
  const { root, slug } = m;
  const main = m.side.mainBranch;
  if (m.side.verify.length > 0) {
    const check: Check = { ...emptyCheck(), id: "verify", name: `looper verify after ${into}`, run: m.side.verify, timeout: CATCH_UP_VERIFY_S };
    const verdicts = await runReported(root, [check], m.checkLogs, m.units, m.project, m.progress, []);
    const red = verdicts.find((v) => !v.green);
    if (red !== undefined) {
      abortMerge(root);
      return {
        kind: "landing",
        verdict: `landing: \`looper verify\` is red after ${into}: ${red.why}\nIts log: ${red.log ?? "none"}\nIn this worktree, run \`git merge --no-ff ${main}\`, make verify green, and commit.`,
      };
    }
  }
  if (mergeUnderWay(root)) {
    try {
      git(root, ["commit", "-q", "-m", `chore(side): side/${slug} catches up with ${main}`]);
    } catch (e) {
      abortMerge(root);
      return { kind: "hold", why: `catch-up: committing the merge failed: ${(e as Error).message}` };
    }
  }
  return { kind: "land", said: `${slug} is retired, and side/${slug} has caught up with ${main}` };
}

/** The chain at merge stage `stage` of `path`: 1 the base, 2 the side's, 3 main's. A side without the file has an empty chain. */
function chainAt(root: string, path: string, stage: number): Chain {
  let text: string;
  try {
    text = git(root, ["show", `:${stage}:${path}`]);
  } catch {
    return { looper: 1, chain: [], floor: [] };
  }
  return parseChain(path, text);
}

/**
 * Resolves a merge whose `files` are all generated: takes main's side of each, renders the README at
 * `readme`, runs `[merge] render` and stages what it all wrote. Returns why it could not, as a landing
 * session's verdict, or null.
 */
async function renderGenerated(m: Merging, files: string[], readme: string, into: string): Promise<string | null> {
  const { root, cfg } = m;
  const main = m.side.mainBranch;
  let chain: Chain | null = null;
  for (const path of files) {
    if (path === readme) {
      try {
        chain = mergedChain(chainAt(root, path, 1), chainAt(root, path, 2), chainAt(root, path, 3));
      } catch (e) {
        return `landing: ${into} conflicts in ${readme}, whose chain does not parse: ${(e as Error).message}\nIn this worktree, run \`git merge --no-ff ${main}\` and resolve it.`;
      }
    } else if (gitOk(root, ["checkout", "--theirs", "--", path])) {
      git(root, ["add", "--", path]);
    } else {
      // Main's side deleted the file.
      git(root, ["rm", "-q", "-f", "--", path]);
    }
  }
  const goals = join(root, cfg.project.goals_dir);
  writeFile(join(goals, README), renderReadme(chain ?? loadChain(goals), goals));
  for (const [i, argv] of cfg.merge.render.entries()) {
    const check: Check = { ...emptyCheck(), id: `merge-render-${i + 1}`, name: `[merge] render \`${argv.join(" ")}\` after ${into}`, run: argv };
    const v = await runCheck(root, check, m.checkLogs, m.project);
    if (!v.green) {
      return `landing: the \`[merge] render\` command \`${argv.join(" ")}\` failed after ${into}: ${v.why}\nIts log: ${v.log ?? "none"}\nIn this worktree, run \`git merge --no-ff ${main}\`, resolve the conflicts, and commit.`;
    }
  }
  git(root, ["add", "-u"]);
  const untracked = git(root, ["ls-files", "--others", "--exclude-standard"]).split("\n").map((l) => l.trim());
  for (const path of untracked.filter((p) => p !== "" && mergeDeclares(cfg.merge, p))) git(root, ["add", "--", path]);
  const left = conflicted(root);
  if (left.length > 0) {
    return `landing: ${into} still conflicts after rendering, in these files:\n${left.join("\n")}\nIn this worktree, run \`git merge --no-ff ${main}\` and resolve them.`;
  }
  return null;
}

/**
 * The chain of a goals README both sides changed: `theirs`, the main branch's, with the side branch's
 * own changes since `base` laid over it. A slug or floor check `ours` dropped is dropped, and one it
 * added is added. An added check whose id the floor holds is renamed `<from>-<id>`, as a retirement
 * names it, so no side's check is lost.
 */
export function mergedChain(base: Chain, ours: Chain, theirs: Chain): Chain {
  const has = (list: Check[], c: Check) => list.some((x) => Bun.deepEquals(x, c));
  const chain = theirs.chain.filter((s) => ours.chain.includes(s) || !base.chain.includes(s));
  for (const s of ours.chain) if (!base.chain.includes(s) && !chain.includes(s)) chain.push(s);
  const floor = theirs.floor.filter((c) => has(ours.floor, c) || !has(base.floor, c));
  for (const c of ours.floor) {
    if (has(base.floor, c) || has(floor, c)) continue;
    const check = { ...c };
    if (floor.some((f) => f.id === check.id)) check.id = `${check.from ?? "side"}-${check.id}`;
    floor.push(check);
  }
  return { ...theirs, chain, floor };
}
