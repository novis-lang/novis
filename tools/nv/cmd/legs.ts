// `bun nv legs`: the two Linux legs by hand, over a goal's fixtures and suites.
//
//     bun nv legs [--goal <slug>] [--full]
//
// The goal is `--goal`'s, else the one `LOOPER_GOAL` names, else the current one (`lib/chain.ts`
// `namedPlan`). Its `env.wsl.targetDir`, `env.valgrind.skip`, fixture list and checks build the legs'
// options through `legsOptions`, and `linuxLegs` runs them. A leg the change does not reach is judged
// from the selection store; `--full` runs both whatever changed, which is how the floor gate runs them
// (`cmd/sweep.ts`). A leg that runs is recorded green or red in the store, whose tree stays where it is
// (`PlanSweep.discard`). The run takes the sweep lock, and on Linux the local origin.
//
// On a host where the legs do not apply, Windows with no WSL or a goal with no WSL target, or Linux with
// no valgrind, it says so and exits 0. Otherwise it exits 0 when both legs are green, 1 when one is red,
// and 2 on a usage error.

import type { Check } from "../accept/accept.ts";
import { type LegName, type LegsSeams, legsOptions, linuxLegs } from "../accept/legs.ts";
import { holdOrigin } from "../accept/origin.ts";
import { PlanSweep } from "../accept/runner.ts";
import { stageLabels } from "../accept/sweep.ts";
import { needLine, type SharedNeed, sharedResources, takeSweepLock } from "../accept/sweep-lock.ts";
import { type Goal, namedPlan } from "../lib/chain.ts";

export const summary = "the two Linux legs by hand, over a goal's fixtures and suites: nv legs [--goal <slug>] [--full]";

const USAGE = "usage: bun nv legs [--goal <slug>] [--full]";

/** What the store says of each leg, and what it records of one that ran. */
export interface LegStore {
  reached(leg: LegName): boolean;
  green(leg: LegName): void | Promise<void>;
  red(leg: LegName): void;
  /** Closes the store without moving its tree. */
  done(): void;
}

/** Everything the command reads or starts, replaced under test. Each one left out is the real one. */
export interface LegsCmdSeams {
  plan(slug: string | undefined): { slug: string; goal: Goal } | null;
  store(plan: Check[], full: boolean, label: (n: number) => string): Promise<LegStore>;
  /** Takes what `need` names and returns its release. */
  hold(need: SharedNeed, platform: NodeJS.Platform): Promise<() => void>;
  machine: Partial<LegsSeams>;
  say(line: string): void;
}

async function sweepStore(plan: Check[], full: boolean, label: (n: number) => string): Promise<LegStore> {
  const sweep = await PlanSweep.open(plan, label, { full });
  return {
    reached: (leg) => sweep.legReached(leg),
    green: (leg) => sweep.legGreen(leg),
    red: (leg) => sweep.legRed(leg),
    done: () => sweep.discard(),
  };
}

/** The sweep lock, and on Linux the origin, which the WSL leg holds inside the distro on Windows. */
async function holdShared(need: SharedNeed, platform: NodeJS.Platform): Promise<() => void> {
  console.log(needLine(need));
  const lock = need.lock ? await takeSweepLock({ note: (t) => console.log(t) }) : null;
  const origin = need.origin && platform !== "win32" ? await holdOrigin() : null;
  if (origin !== null) console.log(origin.line);
  return () => {
    origin?.close();
    lock?.release();
  };
}

const REAL: LegsCmdSeams = { plan: namedPlan, store: sweepStore, hold: holdShared, machine: {}, say: (l) => console.log(l) };

/**
 * Both legs over `goal`'s fixtures and suites, each recorded green or red in the store, whose tree stays
 * where it is. A leg the change does not reach is judged from the store unless `full` is set. Returns the
 * red line, or "" when both legs are green, skipped or not reached. `bun nv sweep` runs the legs here
 * with `full` set when the floor gate opens.
 */
export async function legsOver(goal: Goal, full: boolean, over: Partial<LegsCmdSeams> = {}): Promise<string> {
  const s: LegsCmdSeams = { ...REAL, ...over };
  const checks = goal.checks as Check[];
  const label = stageLabels(goal);
  const store = await s.store(checks, full, label);
  try {
    const o = legsOptions(goal, checks, {
      reached: (leg) => full || store.reached(leg),
      onGreen: (leg) => store.green(leg),
      onRed: (leg) => store.red(leg),
      label,
      onRun: (what) => s.say(`  .. ${what}`),
      seams: s.machine,
    });
    const release = await s.hold(sharedResources(o.programs, true, o.reached), s.machine.platform ?? process.platform);
    try {
      return await linuxLegs(o);
    } finally {
      release();
    }
  } finally {
    store.done();
  }
}

/** Parses, runs the legs and prints; returns the exit code. */
export async function legs(args: string[], over: Partial<LegsCmdSeams> = {}): Promise<number> {
  const s: LegsCmdSeams = { ...REAL, ...over };
  let slug: string | undefined;
  let full = false;
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--full") full = true;
    else if (a === "--goal" && args[i + 1] !== undefined) slug = args[++i];
    else {
      console.error(`${USAGE}\nnv legs: error: ${a === "--goal" ? "--goal needs a slug" : `unexpected argument ${a}`}`);
      return 2;
    }
  }
  const plan = s.plan(slug);
  if (plan === null) {
    console.error(`${USAGE}\nnv legs: error: ${slug ? `no goal record for ${slug}` : "no goal is named or current"}`);
    return 2;
  }
  const red = await legsOver(plan.goal, full, s);
  if (red !== "") {
    s.say(`RED ${red}`);
    return 1;
  }
  s.say(`GREEN the Linux legs of ${plan.slug}`);
  return 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return legs(args);
}
