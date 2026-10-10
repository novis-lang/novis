import { afterEach, beforeEach, expect, test } from "bun:test";
import { type Goal, LOOPER_GOAL_ENV, LOOPER_STAGE_ENV } from "../lib/chain.ts";
import { write } from "../lib/store.ts";
import { chain } from "../schema/chain.ts";
import { goal } from "../schema/goal.ts";
import { handoff } from "../schema/handoff.ts";
import { rule } from "../schema/rule.ts";
import { topic } from "../schema/topic.ts";
import { pack } from "../cmd/pack.ts";
import { type Scratch, scratch } from "./scratch.ts";

// A tree with two rules, one shape, a plan with its fields and one milestone. Goal `g` names the first
// rule, the shape, a plan field and the milestone, and its stage 2 adds the second rule. Goal `bare`
// names nothing, and it is the live goal, so a pack that ignored the goal it was given prints nothing.
const STATUS = { status: "s", done: "d", onDisk: "o", toolchain: "t", adrSlicesLanded: "a", openNow: "the open work", blocking: "nothing blocks" };

const record = (context: Goal["context"], stageContext?: Goal["context"]): Goal => ({
  title: "t",
  milestone: null,
  files: [],
  context,
  stages: [
    { number: 1, title: "one", summary: "s" },
    { number: 2, title: "two", summary: "s", ...(stageContext ? { context: stageContext } : {}) },
  ],
  checks: [],
  env: {},
});

let s: Scratch;
let saved: [string | undefined, string | undefined];

beforeEach(() => {
  s = scratch();
  write(chain, "chain", { live: "bare", goals: ["bare", "g"] }, s.root);
  write(goal, "g", record({ rules: ["alpha/one"], shapes: ["A commit message"], plan: ["Open now"], milestones: ["M1:verify"] }, { rules: ["alpha/two"] }), s.root);
  write(goal, "bare", record({}), s.root);
  write(handoff, "g", { state: "s", next: { stage: 2, files: [], items: [] }, backlog: [] }, s.root);
  write(topic, "alpha", { title: "Alpha", order: 1, rules: ["alpha/one", "alpha/two"] }, s.root);
  write(rule, "alpha/one", { title: "The first rule", status: "shipped", because: [], seeAlso: [], guardedBy: [] }, s.root);
  write(rule, "alpha/two", { title: "The second rule", status: "shipped", because: [], seeAlso: [], guardedBy: [] }, s.root);
  s.put("docs/rules/alpha/one.md", "The first rule's fragment.\n");
  s.put("docs/rules/alpha/two.md", "The second rule's fragment.\n");
  s.put("docs/agent/conventions.md", "# Conventions\n\n## A commit message\n\nThe commit shape.\n\n## A code comment\n\nNot asked for.\n");
  s.put("data/plan/status.json", `${JSON.stringify(STATUS, null, 2)}\n`);
  s.put("data/plan/milestones/M1.json", `${JSON.stringify({ title: "First", order: 1, loopDays: "1", state: "open" }, null, 2)}\n`);
  s.put("docs/plan/m1.md", "# M1 — First\n\nThe scope.\n\n**Verify:** the first milestone runs.\n");
  saved = [process.env[LOOPER_GOAL_ENV], process.env[LOOPER_STAGE_ENV]];
  delete process.env[LOOPER_GOAL_ENV];
  delete process.env[LOOPER_STAGE_ENV];
});

afterEach(() => {
  for (const [name, value] of [[LOOPER_GOAL_ENV, saved[0]], [LOOPER_STAGE_ENV, saved[1]]] as const) {
    if (value === undefined) delete process.env[name];
    else process.env[name] = value;
  }
  s.cleanup();
});

test("nv pack prints the rules, shapes, plan fields and milestones of the goal and its stage", async () => {
  const r = await pack(["--goal", "g", "--stage", "2"], s.root);
  expect(r.code).toBe(0);
  expect(r.text).toContain("== THE RULES THIS GOAL LIVES INSIDE");
  expect(r.text).toContain("---- rule:alpha/one");
  expect(r.text).toContain("The first rule's fragment.");
  expect(r.text).toContain("---- rule:alpha/two");
  expect(r.text).toContain("== THE SHAPES YOU ARE ABOUT TO WRITE");
  expect(r.text).toContain("The commit shape.");
  expect(r.text).not.toContain("Not asked for.");
  expect(r.text).toContain("Open now: the open work");
  expect(r.text).not.toContain("nothing blocks");
  expect(r.text).toContain("-- M1 acceptance (docs/plan/m1.md)");
  expect(r.text).toContain("the first milestone runs");
  expect(r.text).not.toContain("TRAPS");
  const order = ["THE RULES", "THE MAP", "THE SHAPES", "THE PLAN", "THE MILESTONES", "THE NEXT FREE NUMBER"].map((t) => r.text.indexOf(`== ${t}`));
  expect(order.every((at, i) => at >= 0 && (i === 0 || at > order[i - 1]!))).toBe(true);

  const stage1 = await pack(["--goal", "g", "--stage", "1"], s.root);
  expect(stage1.text).toContain("---- rule:alpha/one");
  expect(stage1.text).not.toContain("rule:alpha/two");
});

test("nv pack takes the goal and the stage from LOOPER_GOAL and LOOPER_STAGE when no flag names them", async () => {
  process.env[LOOPER_GOAL_ENV] = "g";
  process.env[LOOPER_STAGE_ENV] = "1";
  const fromEnv = await pack([], s.root);
  expect(fromEnv.code).toBe(0);
  expect(fromEnv.text).toContain("---- rule:alpha/one");
  expect(fromEnv.text).not.toContain("rule:alpha/two");

  // A flag wins over the variable, and with no stage named anywhere the handoff's next stage applies.
  expect((await pack(["--stage", "2"], s.root)).text).toContain("---- rule:alpha/two");
  delete process.env[LOOPER_STAGE_ENV];
  expect((await pack([], s.root)).text).toContain("---- rule:alpha/two");
});

test("nv pack prints no section for a goal whose context is empty", async () => {
  const live = await pack([], s.root);
  expect(live.code).toBe(0);
  expect(live.text).not.toContain("== ");
  expect(live.text).toContain("bare");
  expect((await pack(["--goal", "nope"], s.root)).code).toBe(2);
  expect((await pack(["--stage", "x"], s.root)).code).toBe(2);
});
