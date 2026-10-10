import { afterEach, describe, expect, test } from "bun:test";
import { chainGoals, currentPlan, gatesInFrontOfGroups, type Goal, goalPlan, liveGoal, liveStage, namedPlan, SIDE_ENV, sideGoal, sidePlan } from "../../lib/chain.ts";
import { goneCitations } from "../../cmd/chain.ts";
import { load, write } from "../../lib/store.ts";
import { chain } from "../../schema/chain.ts";
import { scratch, type Scratch } from "../../test/scratch.ts";
import { setLive } from "../old-chain.ts";

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
  /** A looper goal file with front matter `fm`, written as JSON, which is YAML too. */
  const goalFile = (fm: object) => `---\n${JSON.stringify({ looper: 1, ...fm })}\n---\n\n# t\n`;
  const one = (stage: number, check: string, extra: object = {}) =>
    goalFile({ title: "t", stages: [{ id: stage, title: "x", status: "open" }], checks: [{ id: check, stage, name: "n", run: [check] }], extra });

  function seed(chainSlugs: string[] = ["b", "c"]): Scratch {
    const s = scratch();
    s.put("docs/goals/README.md", goalFile({ chain: chainSlugs, floor: [] }));
    s.put("docs/goals/b.md", one(2, "tb", { milestone: "M1" }));
    s.put("docs/goals/c.md", one(2, "tc"));
    return s;
  }

  test("the live goal is the chain's first, and its plan is its own front matter with no check of another goal", () => {
    tmp = seed();
    expect(chainGoals(tmp.root).map((c) => [c.slug, c.num, c.milestone, c.md])).toEqual([
      ["b", 1, "M1", "docs/goals/b.md"],
      ["c", 2, null, "docs/goals/c.md"],
    ]);
    expect(liveGoal(undefined, tmp.root)?.slug).toBe("b");
    expect(goalPlan("c", tmp.root)!.checks).toEqual([{ id: "tc", kind: "command", stage: 2, name: "n", argv: ["tc"] }]);
    expect(goalPlan("gone", tmp.root)).toBeNull();
  });

  test("a check's kind follows from its fields, and Novis's own fields come from `extra`", () => {
    tmp = seed();
    const fm = {
      title: "t",
      context: ["a.rs"],
      stages: [
        { id: 1, title: "one", status: "done" },
        { id: 2, title: "two", status: "open", context: ["b.rs"], extra: { summary: "s", context: { rules: ["r/x"] } } },
      ],
      checks: [
        { id: "cargo", stage: 2, run: ["cargo", "test", "--test", "t"], tests: ["a_test"] },
        { id: "suite", stage: 2, run: ["bun", "nv", "suite", "tests/c/", "--case", "tests/c/x.nvst"], tests: ["tests/c/x.nvst"] },
        { id: "cmd", stage: 2, run: ["bun", "nv", "x"], output_ordered: ["one", "two"], exit: "nonzero", cwd: "." },
        { id: "out", stage: 2, run: ["bun", "nv", "y"], stdout_contains: ["z"] },
        { id: "size", stage: 2, file: "f.json", min_bytes: 10 },
        { id: "text", stage: 2, file: "f.json", output_contains: ["q"] },
      ],
      extra: { milestone: "M2", sweep: "done", files: ["f.nvs"], env: { wsl: { targetDir: "/t" } }, context: { rules: ["r/y"] } },
    };
    tmp.put("docs/goals/d.md", goalFile(fm));
    const plan = goalPlan("d", tmp.root)!;
    expect(plan.checks).toEqual([
      { id: "cargo", kind: "cargo-named", stage: 2, args: ["test", "--test", "t"], tests: ["a_test"] },
      { id: "suite", kind: "nvs-suite", stage: 2, args: ["test", "tests/c/"], cases: ["tests/c/x.nvst"] },
      { id: "cmd", kind: "command", stage: 2, cwd: ".", exit: "nonzero", argv: ["bun", "nv", "x"], want: ["one", "two"] },
      { id: "out", kind: "command", stage: 2, argv: ["bun", "nv", "y"], stdoutContains: ["z"] },
      { id: "size", kind: "min-bytes", stage: 2, file: "f.json", minBytes: 10 },
      { id: "text", kind: "contains", stage: 2, file: "f.json", stdoutContains: ["q"] },
    ]);
    expect([plan.milestone, plan.sweep, plan.files, plan.env]).toEqual(["M2", "done", ["f.nvs"], { wsl: { targetDir: "/t" } }]);
    expect(plan.context).toEqual({ rules: ["r/y"], modules: ["a.rs"] });
    expect(plan.stages[1]).toEqual({ number: 2, title: "two", summary: "s", status: "open", context: { rules: ["r/x"], modules: ["b.rs"] } });
    expect(liveStage("d", tmp.root)).toBe(2);
  });

  test("a side goal is a goal file off the chain, and its plan is its own with a WSL target of its own", () => {
    tmp = seed();
    tmp.put("docs/goals/s.md", one(2, "ts", { env: { wsl: { targetDir: "/var/tmp/t" } } }));
    expect(sidePlan("s", tmp.root)!.checks.map((c) => [c.id, c.stage])).toEqual([["ts", 2]]);
    expect(sidePlan("s", tmp.root)!.env.wsl?.targetDir).toBe("/var/tmp/t-side-s");
    expect(namedPlan("s", tmp.root)!.goal.env.wsl?.targetDir).toBe("/var/tmp/t-side-s");
    expect(namedPlan("b", tmp.root)!.goal.checks.map((c) => c.id)).toEqual(["tb"]);
    expect(sidePlan("gone", tmp.root)).toBeNull();
  });

  test("a process runs a side goal only when the variable names a goal file off the chain", () => {
    tmp = seed();
    const before = process.env[SIDE_ENV];
    try {
      process.env[SIDE_ENV] = "s";
      expect(sideGoal(tmp.root)).toBeNull();
      tmp.put("docs/goals/s.md", one(2, "ts"));
      expect(sideGoal(tmp.root)).toBe("s");
      process.env[SIDE_ENV] = "b";
      expect(sideGoal(tmp.root)).toBeNull();
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
      tmp.put("docs/goals/s.md", one(2, "ts"));
      expect(plan()).toEqual(["s", ["ts"]]);
      delete process.env[SIDE_ENV];
      tmp.put("docs/goals/README.md", goalFile({ chain: ["gone", "b"], floor: [] }));
      expect(plan()).toBeNull();
    } finally {
      if (before === undefined) delete process.env[SIDE_ENV];
      else process.env[SIDE_ENV] = before;
    }
  });
});

describe("the old driver's chain", () => {
  test("setLive moves the pointer, keeps the order, and refuses a goal that is not on the chain", () => {
    tmp = scratch();
    write(chain, "chain", { live: "b", goals: ["a", "b", "c"] }, tmp.root);
    expect(setLive("c", tmp.root)).toBe("data/chain.json");
    expect(load(chain, tmp.root)[0]!.value).toEqual({ live: "c", goals: ["a", "b", "c"] });
    expect(() => setLive("gone", tmp!.root)).toThrow("not on the chain");
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
