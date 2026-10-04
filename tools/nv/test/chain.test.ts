import { afterEach, describe, expect, test } from "bun:test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chainGoals, currentPlan, gatesInFrontOfGroups, type Goal, goalPlan, liveGoal, setLive, SIDE_ENV, sideGoal, sidePlan, walkedGoals } from "../lib/chain.ts";
import { goneCitations } from "../cmd/chain.ts";
import { load, write } from "../lib/store.ts";
import { chain } from "../schema/chain.ts";
import { goal, sideGoal as sideGoalType } from "../schema/goal.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch | undefined;
afterEach(() => {
  tmp?.cleanup();
  tmp = undefined;
});

function g(stages: Goal["stages"], checks: Goal["checks"], files: string[] = [], skip?: string[]): Goal {
  return { title: "t", milestone: null, files, context: {}, stages, checks, env: skip ? { valgrind: { skip } } : {} };
}

const cmd = (id: string, stage: number, argv: string[]) => ({ id, stage, kind: "command" as const, argv });

describe("gatesInFrontOfGroups", () => {
  const stage = [{ number: 2, title: "the work", summary: "s" }];
  const gate = g(stage, [cmd("gate", 2, ["bun", "nv", "proofs", "--gate"])]);
  const group = g(stage, [cmd("group", 2, ["bun", "nv", "proofs", "--verify", "--group", "tools:x"])]);
  const other = g(stage, [cmd("other", 2, ["cargo", "fmt"])]);

  test("a whole-roster gate is named with every goal behind it that still proves a roster group", () => {
    const found = gatesInFrontOfGroups([
      { slug: "owed", goal: gate },
      { slug: "other", goal: other },
      { slug: "x", goal: group },
      { slug: "cards", goal: gate },
      { slug: "y", goal: group },
    ]);
    expect(found).toEqual([
      { slug: "owed", behind: ["x", "y"] },
      { slug: "cards", behind: ["y"] },
    ]);
  });

  test("a gate behind every group goal, and a gate narrowed to part of the roster, are left alone", () => {
    const narrowed = g(stage, [cmd("one", 2, ["bun", "nv", "proofs", "--gate", "--only", "tools:x/a"]), cmd("grp", 2, ["bun", "nv", "proofs", "--gate", "--group", "tools:x"])]);
    expect(
      gatesInFrontOfGroups([
        { slug: "narrowed", goal: narrowed },
        { slug: "x", goal: group },
        { slug: "owed", goal: gate },
        { slug: "later", goal: gate },
      ]),
    ).toEqual([]);
  });
});

describe("the chain's live goal", () => {
  function seed(): Scratch {
    const s = scratch();
    write(chain, "chain", { live: "b", goals: ["a", "b", "c"] }, s.root);
    write(goal, "a", g([{ number: 1, title: "x", summary: "s" }], [cmd("ta", 1, ["a"])]), s.root);
    write(goal, "b", g([{ number: 2, title: "y", summary: "s" }], [cmd("tb", 2, ["b"])]), s.root);
    write(goal, "c", g([{ number: 2, title: "z", summary: "s" }], [cmd("tc", 2, ["c"])]), s.root);
    return s;
  }

  test("the live goal is data/chain.json's `live`, and its plan is its own record with no check of a goal in front of it", () => {
    tmp = seed();
    expect(liveGoal(undefined, tmp.root)?.slug).toBe("b");
    expect(goalPlan("b", tmp.root)).toEqual(load(goal, tmp.root).find((r) => r.id === "b")!.value);
    expect(goalPlan("c", tmp.root)!.checks.map((c) => c.id)).toEqual(["tc"]);
    expect(goalPlan("gone", tmp.root)).toBeNull();
  });

  test("the walked goals are the ones in front of the live goal, and none while no goal is live", () => {
    tmp = seed();
    expect([...walkedGoals(chainGoals(tmp.root), liveGoal(undefined, tmp.root)).keys()]).toEqual(["a"]);
    expect(walkedGoals(chainGoals(tmp.root), null).size).toBe(0);
  });

  test("setLive moves the pointer, keeps the order, and refuses a goal that is not on the chain", () => {
    tmp = seed();
    expect(setLive("c", tmp.root)).toBe("data/chain.json");
    expect(load(chain, tmp.root)[0]!.value).toEqual({ live: "c", goals: ["a", "b", "c"] });
    expect(() => setLive("gone", tmp!.root)).toThrow("not on the chain");
  });

  test("a side goal's plan is its own record, with no check of a chain goal", () => {
    tmp = seed();
    write(sideGoalType, "s", g([{ number: 2, title: "w", summary: "s" }], [cmd("ts", 2, ["s"])]), tmp.root);
    expect(sidePlan("s", tmp.root)!.checks.map((c) => [c.id, c.stage])).toEqual([["ts", 2]]);
    expect(sidePlan("gone", tmp.root)).toBeNull();
  });

  test("a side goal's WSL leg builds into a target of its own", () => {
    tmp = seed();
    write(sideGoalType, "s", { ...g([{ number: 2, title: "w", summary: "s" }], [cmd("ts", 2, ["s"])]), env: { wsl: { targetDir: "/var/tmp/t" } } }, tmp.root);
    expect(sidePlan("s", tmp.root)!.env.wsl?.targetDir).toBe("/var/tmp/t-side-s");
  });

  test("a process runs a side goal only when the variable names one whose prose is on disk", () => {
    tmp = seed();
    const before = process.env[SIDE_ENV];
    try {
      process.env[SIDE_ENV] = "s";
      expect(sideGoal(tmp.root)).toBeNull();
      mkdirSync(join(tmp.root, "docs/agent/goals/side"), { recursive: true });
      writeFileSync(join(tmp.root, "docs/agent/goals/side/s.md"), "# Side goal — s\n");
      expect(sideGoal(tmp.root)).toBe("s");
      process.env[SIDE_ENV] = "../s";
      expect(sideGoal(tmp.root)).toBeNull();
    } finally {
      if (before === undefined) delete process.env[SIDE_ENV];
      else process.env[SIDE_ENV] = before;
    }
  });

  test("the current plan is the side goal's in a side run and the live goal's otherwise", () => {
    tmp = seed();
    const before = process.env[SIDE_ENV];
    const plan = () => {
      const p = currentPlan(tmp!.root);
      return p === null ? null : [p.slug, p.goal.checks.map((c) => c.id)];
    };
    try {
      delete process.env[SIDE_ENV];
      expect(plan()).toEqual(["b", ["tb"]]);
      process.env[SIDE_ENV] = "s";
      mkdirSync(join(tmp.root, "docs/agent/goals/side"), { recursive: true });
      writeFileSync(join(tmp.root, "docs/agent/goals/side/s.md"), "# Side goal — s\n");
      // Prose with no record is a side run with no plan, and never the live goal's by mistake.
      expect(plan()).toBeNull();
      write(sideGoalType, "s", g([{ number: 2, title: "w", summary: "s" }], [cmd("ts", 2, ["s"])]), tmp.root);
      expect(plan()).toEqual(["s", ["ts"]]);
      delete process.env[SIDE_ENV];
      write(chain, "chain", { live: "gone", goals: ["a", "b", "c", "gone"] }, tmp.root);
      expect(plan()).toBeNull();
    } finally {
      if (before === undefined) delete process.env[SIDE_ENV];
      else process.env[SIDE_ENV] = before;
    }
  });
});

describe("a goal named after it left the chain", () => {
  const known = new Set(["goal-closeout", "side-a"]);
  const text = "See goal `goal-closeout`.\nGoals `xml-tree` and `side-a` decided it.\nThe switch sets `live` to the next goal.\n";

  test("a Markdown file naming a goal off the chain is reported by line", () => {
    expect(goneCitations("docs/agent/commands.md", text, known)).toEqual(["docs/agent/commands.md:2  Goals `xml-tree`"]);
  });

  test("a goal's own files, the frozen records and anything not Markdown are exempt", () => {
    for (const path of ["docs/agent/goals/xml-tree.md", "docs/decisions/0200.md", "tools/nv/cmd/chain.ts"]) {
      expect(goneCitations(path, text, known)).toEqual([]);
    }
  });
});
