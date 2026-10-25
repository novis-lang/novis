// The acceptance sweep over a goal's checks, on the selection store: `sweepOver` runs the checks a change
// reaches, with the origin and the sweep lock when the checks need them, and `sweepIds` is the sweep
// `bun nv affected --run` runs over the check ids it chose. After a session, looper runs the goal's
// checks itself.
//
// `livePlan` is the goal a process works on: the side goal `SIDE_ENV` names, or else the chain's live goal,
// each read through `lib/chain.ts` and nothing else.

import { chainGoals, type Goal, goalPlan, liveGoal, SIDE_ENV, sideGoal, sidePlan } from "../lib/chain.ts";
import { type AcceptanceResult, type Check, acceptance } from "./accept.ts";
import { holdOrigin, type Origin } from "./origin.ts";
import { PlanSweep } from "./runner.ts";
import { needLine, sharedResources, takeSweepLock } from "./sweep-lock.ts";

/** The check's name as the sweep prints it: its own, or its fixture's run line. */
export function nameOf(c: Check): string {
  return c.name ?? `run ${[...(c.args ?? []), c.file ?? ""].join(" ")}`.trimEnd();
}

/** The goal a run works on: the live goal and its place on the chain, or a side goal, whose place is null. */
export interface Current {
  slug: string;
  num: number | null;
}

/**
 * The goal this process works on, its plan and the chain's length, or null once the reason it has none is
 * printed. It is the side goal `SIDE_ENV` names, with `sidePlan`'s plan, or else the live goal, with
 * `goalPlan`'s: the goal file's front matter.
 */
export function livePlan(): { live: Current; goal: Goal; total: number } | null {
  const goals = chainGoals();
  const side = sideGoal();
  if (side !== null) {
    const goal = sidePlan(side);
    if (goal === null) {
      console.error(`nv: the side goal \`${side}\` has no goal file at docs/goals/${side}.md`);
      return null;
    }
    return { live: { slug: side, num: null }, goal, total: goals.length };
  }
  if (process.env[SIDE_ENV]) {
    console.error(`nv: ${SIDE_ENV} names \`${process.env[SIDE_ENV]}\`, and there is no docs/goals/<slug>.md by that name`);
    return null;
  }
  const live = liveGoal(goals);
  if (live === null) {
    console.error("nv: the chain in docs/goals/README.md names no live goal, so there is no plan to read");
    return null;
  }
  const goal = goalPlan(live.slug);
  if (goal === null) {
    console.error(`nv: the live goal \`${live.slug}\` has no goal file at docs/goals/${live.slug}.md`);
    return null;
  }
  return { live, goal, total: goals.length };
}

/** A goal's stage labels, `<number> <title>`, and the bare number for a stage the goal does not name. */
export function stageLabels(goal: Goal): (n: number) => string {
  const labels = new Map(goal.stages.map((s) => [s.number, `${s.number} ${s.title}`]));
  return (n: number) => labels.get(n) ?? String(n);
}

/** How a sweep reports itself: each process as it starts, each check the change reaches as it starts, and any other line. */
export interface Progress {
  run: (what: string) => void;
  trace: (c: Check) => void;
  note: (text: string) => void;
}

/**
 * One acceptance sweep over `checks`, on the selection store: the change since its tree is read once,
 * only the checks it reaches start, every run records, and the store's tree moves past the change when
 * the sweep ends, green or red. `onDone` is called with each check's verdict once the sweep ends, which
 * is how a caller learns them without a second run.
 */
export async function sweepOver(
  goal: Goal,
  checks: Check[],
  labelOf: (n: number) => string,
  o: { full: boolean; collect: boolean; onDone?: (c: Check, green: boolean) => void; progress?: Progress },
): Promise<{ result: AcceptanceResult; secs: number; owed: number }> {
  const started = Date.now();
  const p: Progress = o.progress ?? {
    run: (what) => console.error(`  .. ${what}`),
    trace: (c) => console.error(`  check ${nameOf(c)} [${labelOf(c.stage)}]`),
    note: (text) => console.error(`  ${text}`),
  };
  const sweep = await PlanSweep.open(goal.checks as Check[], labelOf, { full: o.full, onRun: p.run, say: p.note });
  const reachedOf = new Map<string, boolean>();
  const reached = (c: Check) => {
    let r = reachedOf.get(c.id);
    if (r === undefined) reachedOf.set(c.id, (r = sweep.reached(c)));
    return r;
  };
  const n = checks.filter(reached).length;
  const c = sweep.change;
  const since = c.full ? (c.global ?? "everything") : `${c.changes.length} path(s) changed since the recorded tree`;
  p.note(`${since}; ${n} of ${checks.length} check(s) reached, the other ${checks.length - n} not started`);
  // A sweep that uses the origin or a perf guard takes the lock, which another tree's sweep using one of
  // them may hold. The time spent waiting for it is not this sweep's cost.
  const need = sharedResources(checks.filter(reached));
  p.note(needLine(need));
  const asked = Date.now();
  const lock = need.lock ? await takeSweepLock({ note: p.note }) : null;
  const waited = Date.now() - asked;
  let result: AcceptanceResult;
  let owed = 0;
  let origin: Origin | null = null;
  try {
    if (need.origin) {
      origin = await holdOrigin();
      p.note(origin.line);
    }
    result = await acceptance(checks, {
      label: labelOf,
      sweep,
      reached,
      judged: (x) => sweep.judged(x),
      collect: o.collect,
      trace: p.trace,
    });
  } finally {
    origin?.close();
    lock?.release();
    const closed = sweep.close();
    owed = closed.owed;
    const ran = Object.entries(closed.ran).map(([k, n]) => `${n} ${k}`).join(", ");
    p.note(`the sweep ran ${ran || "no atom"}; ${owed} atom(s) the change reached and nothing ran stay owed`);
    if (closed.diverged.size > 0) {
      p.note(`${closed.diverged.size} atom(s) ended differently on the recording run than on the judged run, and run every time until they agree:`);
      for (const [id, why] of [...closed.diverged].sort(([a], [b]) => (a < b ? -1 : 1))) p.note(`  diverged  ${id}: ${why}`);
    }
  }
  for (const x of checks) o.onDone?.(x, result.verdicts.get(x.id) ?? false);
  return { result, secs: Math.round((Date.now() - started - waited) / 1000), owed };
}

/**
 * The sweep over the plan's checks whose ids are in `ids`, every red collected: what `nv affected --run`
 * runs once it has chosen them. Exits 0 when every one is green, 1 when one is red, and 2 when there is
 * no plan.
 */
export async function sweepIds(ids: Set<string>): Promise<number> {
  const found = livePlan();
  if (found === null) return 2;
  const { goal } = found;
  const labelOf = stageLabels(goal);
  const chosen = (goal.checks as Check[]).filter((c) => ids.has(c.id));
  if (chosen.length === 0) return 0;
  console.log(`running ${chosen.length} acceptance ${chosen.length === 1 ? "check" : "checks"} (collecting every red)`);
  const { result, secs } = await sweepOver(goal, chosen, labelOf, { full: false, collect: true });
  console.log(`cost: ${secs}s, ${result.ran} run, ${result.answered} not reached by the change, of ${chosen.length}`);
  if (result.fail !== "") {
    console.log(`NOT GREEN: ${result.fail}`);
    return 1;
  }
  console.log("GREEN: every acceptance check the change reaches passes");
  return 0;
}
