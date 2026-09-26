import { afterEach, describe, expect, test } from "bun:test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { type Goal, goalPlan, graduatesTo, liveGoal, setLive, SIDE_ENV, sideGoal, sidePlan, withFloor } from "../lib/chain.ts";
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

  test("a walked test check the permanent suite runs graduates to one check per crate and per case tree", () => {
    const cargo = (id: string, args: string[], extra = {}) => ({ id, stage: 2, kind: "cargo-named" as const, args, tests: [`${id}_passes`], ...extra });
    const suite = (id: string, path: string, extra = {}) => ({ id, stage: 2, kind: "nvs-suite" as const, args: ["test", path], cases: [`${path}/a.nvst`], ...extra });
    const own = g([{ number: 2, title: "the work", summary: "s" }], [cmd("mine", 2, ["a"])]);
    const walked = g(
      [{ number: 2, title: "x", summary: "s" }],
      [
        cargo("whole", ["test", "-p", "nvs-types"]),
        cargo("one-target", ["test", "-p", "nvs-types", "--test", "atoms"]),
        cargo("lib", ["test", "-p", "nvs-ir", "--lib"]),
        cargo("release", ["test", "--release", "-p", "nvs-ir"]),
        cargo("filtered", ["test", "-p", "nvs-ir", "--", "--ignored"]),
        suite("core", "tests/conformance/core"),
        suite("diff", "tests/differential/"),
        suite("counted", "tests/conformance/", { minPassing: 10 }),
        suite("lsp", "tests/lsp"),
      ],
    );
    const plan = withFloor(own, [{ slug: "old", goal: walked }]);
    expect(plan.checks.map((c) => [c.id, c.stage, c.args])).toEqual([
      ["mine", 2, undefined],
      ["release", 1, ["test", "--release", "-p", "nvs-ir"]],
      ["filtered", 1, ["test", "-p", "nvs-ir", "--", "--ignored"]],
      ["counted", 1, ["test", "tests/conformance/"]],
      ["lsp", 1, ["test", "tests/lsp"]],
      ["permanent-suite-nvs-types", 1, ["test", "-p", "nvs-types"]],
      ["permanent-suite-nvs-ir", 1, ["test", "-p", "nvs-ir"]],
      ["permanent-suite-conformance", 1, ["test", "tests/conformance/"]],
      ["permanent-suite-differential", 1, ["test", "tests/differential/"]],
    ]);
    expect(plan.checks.find((c) => c.id === "permanent-suite-nvs-types")?.tests).toEqual([]);
  });

  test("the goal's own test checks never graduate, and a walked check with a field the suite lacks stays", () => {
    const named = { id: "own-test", stage: 2, kind: "cargo-named" as const, args: ["test", "-p", "nvs-types"], tests: ["t"] };
    const own = g([{ number: 2, title: "the work", summary: "s" }], [named]);
    const kept = { ...named, id: "in-a-dir", cwd: "crates" };
    const plan = withFloor(own, [{ slug: "old", goal: g([{ number: 2, title: "x", summary: "s" }], [kept]) }]);
    expect(graduatesTo(named)?.id).toBe("permanent-suite-nvs-types");
    expect(plan.checks.map((c) => c.id)).toEqual(["own-test", "in-a-dir"]);
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

  test("a side goal's plan carries every goal in front of the live one, not the live goal's own checks", () => {
    tmp = seed();
    write(sideGoalType, "s", g([{ number: 2, title: "w", summary: "s" }], [cmd("ts", 2, ["s"])]), tmp.root);
    expect(sidePlan("s", tmp.root)!.checks.map((c) => [c.id, c.stage])).toEqual([
      ["ts", 2],
      ["ta", 1],
    ]);
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
});
