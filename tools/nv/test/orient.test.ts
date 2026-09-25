import { describe, expect, test } from "bun:test";
import { type GoalValue, locateCheck, manifestFindings } from "../cmd/orient.ts";

describe("locateCheck", () => {
  const g: GoalValue = {
    context: {},
    stages: [
      { number: 1, title: "the floor" },
      { number: 3, title: "the parser" },
    ],
    checks: [
      { id: "named", stage: 3, kind: "command", name: "the parser reads a file" },
      { id: "fixture", stage: 3, kind: "program", file: "tests/x.nvs" },
      { id: "bare", stage: 1, kind: "command" },
      { id: "twin", stage: 1, kind: "command", name: "the parser reads a file" },
    ],
  };
  test("the ledger's label finds the check by its name, its file or its id, inside its own stage", () => {
    expect(locateCheck(g, "the parser reads a file [3 the parser]: exit 1 -- no").map((c) => c.id)).toEqual(["named"]);
    expect(locateCheck(g, "tests/x.nvs [3 the parser]: exit 2").map((c) => c.id)).toEqual(["fixture"]);
    expect(locateCheck(g, "bare [1 the floor]: exit 1").map((c) => c.id)).toEqual(["bare"]);
    expect(locateCheck(g, "the parser reads a file [1 the floor]: exit 1").map((c) => c.id)).toEqual(["twin"]);
    expect(locateCheck(g, "")).toEqual([]);
    expect(locateCheck(g, "something else [3 the parser]: exit 1")).toEqual([]);
  });
});

function goal(context: GoalValue["context"], stages: GoalValue["stages"] = []): GoalValue {
  return { context, stages };
}

describe("manifestFindings", () => {
  test("a goal with no context is a note, and nothing else is read", () => {
    expect(manifestFindings(goal({}), "g", null)).toEqual({ problems: [], notes: ["g: has no `context`, so a session opens on the unscoped pack"] });
  });

  test("a shape conventions.md has no heading for is a problem, and one it has is not", () => {
    const found = manifestFindings(goal({ shapes: ["A commit message", "A shape nobody wrote"] }), "g", null);
    expect(found.problems).toHaveLength(1);
    expect(found.problems[0]).toContain("'A shape nobody wrote'");
  });

  test("a decision record's shape without the rule fragment's is a problem", () => {
    const found = manifestFindings(goal({ shapes: ["A decision record"] }), "g", null);
    expect(found.problems.some((p) => p.includes("without 'A rule fragment'"))).toBe(true);
  });

  test("a playbook selector that reaches no bullet is a problem, and a stage's overlay is audited too", () => {
    const found = manifestFindings(goal({ rules: ["testing/feature-proofs"] }, [{ number: 2, title: "t", context: { playbook: ["Tooling > a lead nobody wrote"] } }]), "g", null);
    expect(found.problems).toHaveLength(1);
    expect(found.problems[0]).toContain("playbook selector 'Tooling > a lead nobody wrote'");
  });

  test("a record or rule that does not exist yet is only a note", () => {
    const found = manifestFindings(goal({ adrs: ["9999"], rules: ["nowhere/no-such-rule"] }), "g", null);
    expect(found.problems).toEqual([]);
    expect(found.notes).toEqual([
      "g: adrs names 9999, and docs/decisions/ has no 9999.md",
      "g: rules names rule:nowhere/no-such-rule, and the rulebook has no rule with that id",
    ]);
  });

  test("three stages and no stage context is a note once the base names a record", () => {
    const stages = [1, 2, 3].map((number) => ({ number, title: "t" }));
    const found = manifestFindings(goal({ adrs: ["0004"] }, stages), "g", null);
    expect(found.notes.some((n) => n.includes("runs 3 stages and narrows to none of them"))).toBe(true);
  });
});
