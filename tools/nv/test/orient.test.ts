import { describe, expect, test } from "bun:test";
import { type GoalValue, manifestFindings } from "../cmd/orient.ts";

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
