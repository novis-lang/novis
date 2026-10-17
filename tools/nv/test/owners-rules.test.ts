// A designed rule's owner: the rule record's checks on it, how `bun nv owners` classifies it, and the
// mark the rendered chapter prints for it.
import { afterEach, describe, expect, test } from "bun:test";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { classifyRules, collectRules } from "../cmd/owners.ts";
import { designedMark } from "../cmd/rules.ts";
import { Index } from "../lib/index.ts";
import { write } from "../lib/store.ts";
import { rule } from "../schema/rule.ts";
import { topic } from "../schema/topic.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

type Owner = { goal?: string };

/** A scratch tree with goal file `kept`, and `rules`. */
function seed(rules: Record<string, { status: "shipped" | "designed" } & Owner>): void {
  tmp = scratch();
  tmp.put("docs/goals/kept.md", "# kept\n");
  for (const [slug, r] of Object.entries(rules)) {
    write(rule, `x/${slug}`, { title: slug, seeAlso: [], guardedBy: [], ...r }, tmp.root);
  }
  write(topic, "x", { title: "X", order: 1, rules: Object.keys(rules).map((s) => `x/${s}`) }, tmp.root);
}

function findings(): string[] {
  const index = new Index({ root: tmp.root, types: [rule, topic], prose: [] });
  index.refresh();
  const out = index.check().map((f) => `${f.path}: ${f.message}`);
  index.close();
  return out;
}

describe("a rule record's owner", () => {
  test("a designed rule names one owner or none, and a shipped rule names none", () => {
    seed({ a: { status: "designed", goal: "kept" }, c: { status: "designed" }, d: { status: "shipped" } });
    expect(findings()).toEqual([]);
  });

  test("a shipped rule that names an owner is a finding", () => {
    seed({ a: { status: "shipped", goal: "kept" } });
    expect(findings()).toEqual([
      "data/rules/x/a.json: a shipped rule names no owner: x/a is shipped and names goal kept",
    ]);
  });
});

describe("nv owners, designed rules", () => {
  test("each owner falls into its kind, and any goal with a file is accepted", () => {
    seed({
      goal: { status: "designed", goal: "kept" },
      gone: { status: "designed", goal: "gone" },
      none: { status: "designed" },
      shipped: { status: "shipped" },
    });
    const found = collectRules(tmp.root);
    expect(found.map((r) => r.id)).toEqual(["x/goal", "x/gone", "x/none"]);
    const kinds = classifyRules(found, tmp.root);
    const ids = Object.fromEntries(Object.entries(kinds).map(([k, rs]) => [k, rs.map((r) => r.id)]));
    expect(ids).toEqual({
      goal: ["x/goal"],
      broken: ["x/gone"],
      untagged: ["x/none"],
    });
    expect(kinds.broken.map((r) => r.why)).toEqual(["no goal `gone` in docs/goals/"]);
  });

  test("deleting the goal file makes its rule's owner broken", () => {
    seed({ a: { status: "designed", goal: "kept" } });
    expect(classifyRules(collectRules(tmp.root), tmp.root).goal.map((r) => r.id)).toEqual(["x/a"]);
    rmSync(join(tmp.root, "docs", "goals", "kept.md"));
    expect(classifyRules(collectRules(tmp.root), tmp.root).broken.map((r) => r.id)).toEqual(["x/a"]);
  });
});

describe("nv rules --render, the designed mark", () => {
  test("names the owner, and says the rule is not in the compiler when it names none", () => {
    expect(designedMark({ owner: "M10" })).toBe("designed — owned by M10");
    expect(designedMark({ owner: "goal request-isolation-debts" })).toBe("designed — owned by goal request-isolation-debts");
    expect(designedMark({ owner: "" })).toBe("designed — not yet in the compiler");
  });
});
