import { afterEach, describe, expect, test } from "bun:test";
import { type Goal, goalPlan, liveGoal, setLive, withFloor } from "../lib/chain.ts";
import { load, write } from "../lib/store.ts";
import { chain } from "../schema/chain.ts";
import { goal } from "../schema/goal.ts";
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

describe("withFloor", () => {
  test("a goal with no floor stage gets stage 1 titled floor, and every walked check lands in it once", () => {
    const own = g([{ number: 2, title: "the work", summary: "s" }], [cmd("mine", 2, ["a"])]);
    const first = g([{ number: 1, title: "x", summary: "s" }], [cmd("fmt", 1, ["cargo", "fmt"]), cmd("t1", 1, ["t"])], ["examples/a.nvs"], ["examples/a.nvs"]);
    const second = g([{ number: 1, title: "floor", summary: "s" }], [cmd("fmt", 1, ["cargo", "fmt"]), cmd("fmt-too", 1, ["cargo", "fmt"])], ["examples/b.nvs"]);
    const plan = withFloor(own, [
      { slug: "first", goal: first },
      { slug: "second", goal: second },
    ]);
    expect(plan.stages.map((s) => [s.number, s.title])).toEqual([
      [1, "floor"],
      [2, "the work"],
    ]);
    expect(plan.checks.map((c) => [c.id, c.stage])).toEqual([
      ["mine", 2],
      ["fmt", 1],
      ["t1", 1],
    ]);
    expect(plan.files).toEqual(["examples/a.nvs", "examples/b.nvs"]);
    expect(plan.env.valgrind?.skip).toEqual(["examples/a.nvs"]);
  });

  test("a goal that keeps its own floor stage keeps it, and a walked check it already carries is not added twice", () => {
    const own = g(
      [
        { number: 1, title: "the floor", summary: "s" },
        { number: 3, title: "the work", summary: "s" },
      ],
      [cmd("fmt", 1, ["cargo", "fmt"]), cmd("mine", 3, ["a"])],
    );
    const walked = g([{ number: 2, title: "x", summary: "s" }], [cmd("fmt", 2, ["cargo", "fmt"]), cmd("mine", 2, ["b"])]);
    const plan = withFloor(own, [{ slug: "old", goal: walked }]);
    expect(plan.stages).toEqual(own.stages);
    expect(plan.checks.map((c) => [c.id, c.stage])).toEqual([
      ["fmt", 1],
      ["mine", 3],
      ["mine--old", 1],
    ]);
  });

  test("a goal whose stage 1 is its own work gets its floor in front of its first stage", () => {
    const own = g([{ number: 1, title: "the work", summary: "s" }], [cmd("mine", 1, ["a"])]);
    const plan = withFloor(own, [{ slug: "old", goal: g([{ number: 1, title: "x", summary: "s" }], [cmd("t", 1, ["t"])]) }]);
    expect(plan.stages[0]).toEqual({ number: 0, title: "floor", summary: "Every check of every walked goal still passes." });
    expect(plan.checks.map((c) => [c.id, c.stage])).toEqual([
      ["mine", 1],
      ["t", 0],
    ]);
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

  test("the live goal is data/chain.json's `live`, and its plan carries every goal in front of it", () => {
    tmp = seed();
    expect(liveGoal(undefined, tmp.root)?.slug).toBe("b");
    expect(goalPlan("b", tmp.root)!.checks.map((c) => c.id)).toEqual(["tb", "ta"]);
    expect(goalPlan("c", tmp.root)!.checks.map((c) => c.id)).toEqual(["tc", "ta", "tb"]);
    expect(goalPlan("gone", tmp.root)).toBeNull();
  });

  test("setLive moves the pointer, keeps the order, and refuses a goal that is not on the chain", () => {
    tmp = seed();
    expect(setLive("c", tmp.root)).toBe("data/chain.json");
    expect(load(chain, tmp.root)[0]!.value).toEqual({ live: "c", goals: ["a", "b", "c"] });
    expect(() => setLive("gone", tmp!.root)).toThrow("not on the chain");
  });
});
