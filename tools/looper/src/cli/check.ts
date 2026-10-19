// `check [--name <text>] [--all]`: how a session proves one acceptance check, on the project in the
// current directory. It ports `crates/looper/src/cmd/check.rs`, and runs its checks as acceptance does
// (`../verify/accept.ts`).
//
// - It runs the live goal's acceptance as the driver does: the floor, the due stages' checks, then the
//   reach checks once all else is green.
// - With `--name`, it runs only the floor and goal checks whose id, or whose name ignoring case,
//   contains the text. Logs go to `.looper/checks/<id>.log`.
// - A check that is a unit of the project's unit store, the one verify keeps, runs only when a change
//   reaches it, and prints one `·` line when its last green still holds.
// - With `--all`, every check runs. A unit that is red although no change reached it prints a `!` line
//   and joins the store's selection-miss list.
// - Every check gets `LOOPER_GOAL`, `LOOPER_STAGE` and `LOOPER_EXTRA`, as it does in the loop.
// - With `--json` its lines go to stderr, and it prints one `Checked`. No live goal, or no check that
//   matches, is a refusal. It exits 1 when any check is red.

import { join, relative } from "node:path";
import { loadConfig } from "../goal/config.ts";
import { liveGoal, loadChain, loadGoal } from "../goal/model.ts";
import { acceptance, named, type Project, runReported, unreported, type Units, type Verdict } from "../verify/accept.ts";
import { storeDir } from "../verify/select.ts";
import type { Mark } from "../verify/verify.ts";
import { goalVars } from "./orient.ts";
import { type Command, type Context, Exit, Refusal, type Report, usage } from "./cli.ts";

/** What `check --json` prints: each check in the order it ran. */
export interface Checked {
  green: boolean;
  checks: CheckRun[];
  /** The ids of the checks not run because a stage is open or another check is red. */
  deferred: string[];
}

/** One check that ran or was kept. */
export interface CheckRun {
  id: string;
  result: Mark;
  /** Why it is red, in one line; empty otherwise. */
  why: string;
  /** The log of its run, relative to the root, when it ran a command. */
  log: string | null;
  /** Whether it is red although no change reached it, which only `--all` finds. */
  missed: boolean;
}

/** Where a project's check logs go. */
export function checkLogs(root: string): string {
  return join(root, ".looper", "checks");
}

function runs(root: string, verdicts: Verdict[]): CheckRun[] {
  return verdicts.map((v) => ({
    id: v.id,
    result: v.kept ? "kept" : v.green ? "green" : "red",
    why: v.why,
    log: v.log === null ? null : relative(root, v.log).replaceAll("\\", "/"),
    missed: v.missed,
  }));
}

/** Prints one line per check and the deferred ones, and gives the report that exits 1 when any is red. */
function report(checked: Checked, ctx: Context): Report {
  for (const c of checked.checks) {
    if (c.result === "kept") ctx.line(`· ${c.id}: kept, nothing it reads changed`);
    else if (c.result === "green") ctx.line(`✓ ${c.id}`);
    else ctx.line(`✗ ${c.id}: ${c.why}`);
    if (c.missed) ctx.line(`! ${c.id}: red, but no change reached it; added to the selection-miss list`);
  }
  if (checked.deferred.length > 0) ctx.line(`(reach deferred: ${checked.deferred.join(", ")})`);
  return { code: checked.green ? Exit.ok : Exit.refused, doc: checked };
}

export const checkCommand: Command = {
  name: "check",
  about: "Runs the live goal's acceptance, or the checks a name matches: `check [--name <text>] [--all]`.",
  json: true,
  run: async (args, ctx) => {
    let text: string | undefined;
    let all = false;
    for (let i = 0; i < args.length; i++) {
      const a = args[i] as string;
      if (a === "--all") all = true;
      else if (a === "--name") {
        text = args[++i];
        if (text === undefined) throw usage("`--name` takes a text");
      } else if (a.startsWith("--name=")) text = a.slice("--name=".length);
      else throw usage(`\`check\` does not take \`${a}\``);
    }
    const root = process.cwd();
    const cfg = loadConfig(root);
    const dir = join(root, cfg.project.goals_dir);
    const chain = loadChain(dir);
    const live = liveGoal(root, dir, chain);
    if (live === undefined) throw new Refusal("no live goal");
    const goal = loadGoal(dir, live);
    const logs = checkLogs(root);
    const units: Units = { store: storeDir(root), all };
    const project: Project = { checks: cfg.checks, vars: goalVars(root, dir) };
    if (text === undefined) {
      const acc = await acceptance(root, goal, chain.floor, logs, units, project, unreported);
      return report({ green: acc.green, checks: runs(root, acc.verdicts), deferred: acc.deferred }, ctx);
    }
    const picked = named(chain.floor, goal, text);
    if (picked.length === 0) throw new Refusal(`no check matches "${text}"`);
    const verdicts = await runReported(root, picked, logs, units, project, unreported, []);
    return report({ green: verdicts.every((v) => v.green), checks: runs(root, verdicts), deferred: [] }, ctx);
  },
};
