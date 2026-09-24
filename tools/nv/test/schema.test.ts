import { describe, expect, test } from "bun:test";
import { s, SchemaError, type Infer } from "../lib/schema.ts";

const goal = s.object({
  slug: s.slug(),
  title: s.string(),
  position: s.int(),
  weight: s.number(),
  live: s.boolean(),
  kind: s.enum("chain", "side"),
  milestone: s.ref("milestone"),
  note: s.optional(s.string()),
  parent: s.nullable(s.ref("goal")),
  files: s.array(s.string()),
  env: s.map(s.string()),
  checks: s.array(s.object({ id: s.slug(), argv: s.array(s.string()) })),
});
type Goal = Infer<typeof goal>;

const valid: Goal = {
  slug: "tooling-overhaul",
  title: "The tools",
  position: 3,
  weight: 1.5,
  live: true,
  kind: "chain",
  milestone: "M9",
  parent: null,
  files: ["a.ts"],
  env: { B: "2", A: "1" },
  checks: [{ id: "selftest", argv: ["bun", "nv", "selftest"] }],
};

describe("schema", () => {
  test("a valid value has no issues, and an optional field may be absent", () => {
    expect(goal.validate(valid)).toEqual([]);
    expect(goal.is({ ...valid, note: "x" })).toBe(true);
  });

  test("every issue is reported with where it is", () => {
    const bad = { ...valid, slug: "Not A Slug", position: 1.5, kind: "other", extra: 1, checks: [{ id: "x" }] } as unknown;
    const at = goal.validate(bad).map((i) => i.at);
    expect(at).toEqual(["slug", "position", "kind", "checks[0].argv", "extra"]);
    const { title: _title, ...missing } = valid;
    expect(goal.validate(missing)).toEqual([{ at: "title", message: "is missing" }]);
  });

  test("an optional field is absent or valid, never undefined", () => {
    expect(goal.is({ ...valid, note: undefined })).toBe(false);
  });

  test("parse throws a SchemaError naming each issue", () => {
    expect(() => goal.parse({ ...valid, live: "yes" })).toThrow(SchemaError);
    expect(goal.parse(valid)).toBe(valid);
  });

  test("ordered puts object keys in declaration order and sorts a map", () => {
    const shuffled = Object.fromEntries(Object.entries(valid).reverse()) as Goal;
    expect(Object.keys(shuffled)[0]).toBe("checks");
    const out = goal.ordered(shuffled);
    expect(Object.keys(out)).toEqual(Object.keys(goal.shape).filter((k) => k in valid));
    expect(Object.keys(out.env)).toEqual(["A", "B"]);
  });

  test("the JSON Schema has the fields, the required list and the references", () => {
    const js = goal.describe("A goal.").jsonSchema() as {
      description: string;
      required: string[];
      additionalProperties: boolean;
      properties: Record<string, Record<string, unknown>>;
    };
    expect(js.description).toBe("A goal.");
    expect(js.additionalProperties).toBe(false);
    expect(js.required).not.toContain("note");
    expect(js.required).toContain("parent");
    expect(js.properties.milestone).toEqual({ type: "string", minLength: 1, "x-references": "milestone" });
    expect(js.properties.kind).toEqual({ enum: ["chain", "side"] });
    expect(js.properties.position).toEqual({ type: "integer" });
  });

  test("describe leaves the original schema without a description", () => {
    const base = s.string();
    base.describe("x");
    expect(base.jsonSchema()).toEqual({ type: "string" });
  });
});
