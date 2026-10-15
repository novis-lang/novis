// `bun nv suite`: one suite of `.nvst` cases, judged the way a goal's `nvs-suite` check is.
//
//     bun nv suite [lsp-test] <tree>... [--case <path>]... [--min-passing <n>] [--full]
//
// The arguments build the `Check` a goal record would hold, `args = ["test", <tree>...]` with its `cases`
// and `minPassing`, and `PlanSweep` runs and judges it (`accept/runner.ts` `suiteVerdict`). A leading
// `lsp-test` makes it `args = ["lsp-test", <tree>...]`, the `.lspt` cases `nvs lsp-test` runs, which the
// sweep runs as one command and judges by its summary line (`accept/accept.ts` `judgeTests`). So a hook
// writes one argv per `nvs-suite` check and needs no goal record. The cases the change reaches run and
// are recorded in the selection store, and every other case is judged from its last recorded run;
// `--full` runs them all. The store's tree is left where it is, because one suite is not a sweep of the
// plan (`PlanSweep.discard`).
//
// The `nvs` is the covws build the runner makes, or the one `NVS_BIN` names. Exits 0 green, 1 red, 2 on
// a usage error. A green run prints `(pass) <path>` for each `--case`, the line looper credits a goal
// check's `tests:` entry from.

import type { Check, Verdict } from "../accept/accept.ts";
import { PlanSweep } from "../accept/runner.ts";

export const summary = "judge one suite of .nvst or .lspt cases as a goal's nvs-suite check: nv suite [lsp-test] <tree>... [--case <path>]... [--min-passing <n>] [--full]";

const USAGE = "usage: bun nv suite [lsp-test] <tree>... [--case <path>]... [--min-passing <n>] [--full]";

/** A tree `nvs test` runs as a suite: one directory directly under `tests/`. */
const TREE = /^tests\/[^/]+\/?$/;

export interface SuiteSeams {
  /** Runs and judges the check. */
  judge(c: Check, full: boolean): Promise<Verdict>;
  /** Prints one line. */
  say(line: string): void;
}

/** The `nvs-suite` check the arguments name, with whether to run every case, or a usage error. */
export function suiteCheck(args: string[]): { check: Check; full: boolean } | { error: string } {
  const trees: string[] = [];
  const cases: string[] = [];
  let minPassing: number | undefined;
  let full = false;
  const runner = args[0] === "lsp-test" ? "lsp-test" : "test";
  for (let i = runner === "test" ? 0 : 1; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--full") full = true;
    else if (a === "--case" || a === "--min-passing") {
      const v = args[++i];
      if (v === undefined) return { error: `${a} needs a value` };
      if (a === "--case") cases.push(v.replace(/\\/g, "/"));
      else if (/^\d+$/.test(v)) minPassing = Number(v);
      else return { error: `--min-passing takes a whole number, not ${v}` };
    } else if (a.startsWith("-")) return { error: `unknown flag ${a}` };
    else {
      const tree = a.replace(/\\/g, "/");
      if (!TREE.test(tree)) return { error: `${a} is not a tree under tests/` };
      trees.push(tree.replace(/\/+$/, ""));
    }
  }
  if (trees.length === 0) return { error: "name at least one tree" };
  const check: Check = { id: "nv-suite", kind: "nvs-suite", stage: 0, name: `nvs ${runner} ${trees.join(" ")}`, args: [runner, ...trees] };
  if (cases.length > 0) check.cases = cases;
  if (minPassing !== undefined) check.minPassing = minPassing;
  return { check, full };
}

/** One suite through `PlanSweep`, whose store keeps every case it ran. */
async function sweepJudge(c: Check, full: boolean): Promise<Verdict> {
  const nvs = process.env.NVS_BIN;
  const sweep = await PlanSweep.open([c], () => "nv suite", { full, ...(nvs ? { nvs } : {}), onRun: (what) => console.log(`  ${what}`) });
  try {
    return await sweep.check(c);
  } finally {
    sweep.discard();
  }
}

/** Parses, judges and prints; returns the exit code. */
export async function suite(args: string[], seams: SuiteSeams = { judge: sweepJudge, say: (l) => console.log(l) }): Promise<number> {
  const got = suiteCheck(args);
  if ("error" in got) {
    console.error(`${USAGE}\nnv suite: error: ${got.error}`);
    return 2;
  }
  const v = await seams.judge(got.check, got.full);
  if (v.fail !== "") seams.say(`RED ${v.fail}`);
  else if (v.short !== "") seams.say(`SHORT ${v.short}`);
  else {
    for (const kase of got.check.cases ?? []) seams.say(`(pass) ${kase}`);
    seams.say(`GREEN ${got.check.name} [nv suite]`);
  }
  return v.fail === "" && v.short === "" ? 0 : 1;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return suite(args);
}
