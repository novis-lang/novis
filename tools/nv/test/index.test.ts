import { afterEach, describe, expect, test } from "bun:test";
import { rmSync, utimesSync } from "node:fs";
import { join } from "node:path";
import { Index } from "../lib/index.ts";
import { defineRecord, s } from "../lib/schema.ts";
import { write } from "../lib/store.ts";
import { scratch, type Scratch } from "./scratch.ts";

const milestone = defineRecord({ name: "milestone", dir: "plan/milestones", schema: s.object({ title: s.string() }) });
const goal = defineRecord({
  name: "goal",
  dir: "goals",
  schema: s.object({
    milestone: s.ref("milestone"),
    live: s.boolean(),
    tags: s.array(s.string()),
    checks: s.array(s.object({ id: s.slug(), stage: s.int() })),
  }),
  views: { live_goal: "SELECT id FROM goal WHERE live = 1" },
  checks: [{ name: "a goal has a check", sql: "SELECT path, id || ' has none' AS detail FROM goal g WHERE NOT EXISTS (SELECT 1 FROM goal__checks c WHERE c.owner = g.id)" }],
});
const chain = defineRecord({ name: "chain", dir: "chain", single: true, schema: s.array(s.ref("goal")) });
const TYPES = [milestone, goal, chain];

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

function seed(): Index {
  tmp = scratch();
  write(milestone, "M9", { title: "Tooling" }, tmp.root);
  write(goal, "tools", { milestone: "M9", live: true, tags: ["a", "b"], checks: [{ id: "selftest", stage: 3 }] }, tmp.root);
  write(chain, "chain", ["tools"], tmp.root);
  tmp.put("docs/a.md", "# A\n\nSee `rule:x/y`.\n");
  return new Index({ root: tmp.root, types: TYPES, prose: ["docs/**/*.md"] });
}

describe("index", () => {
  test("a rebuild builds a table per type, child tables per list, and the views", () => {
    const index = seed();
    expect(index.refresh()).toEqual({ read: 4, unchanged: 0, removed: 0 });
    expect(index.query("SELECT id, milestone, live FROM goal")).toEqual([{ id: "tools", milestone: "M9", live: 1 }]);
    expect(index.query("SELECT value FROM goal__tags ORDER BY ord")).toEqual([{ value: "a" }, { value: "b" }]);
    expect(index.query("SELECT owner, id, stage FROM goal__checks")).toEqual([{ owner: "tools", id: "selftest", stage: 3 }]);
    expect(index.query("SELECT value FROM chain__items")).toEqual([{ value: "tools" }]);
    expect(index.query("SELECT * FROM live_goal")).toEqual([{ id: "tools" }]);
    expect(index.query("SELECT type, id FROM records ORDER BY type")).toEqual([
      { type: "chain", id: "chain" },
      { type: "goal", id: "tools" },
      { type: "milestone", id: "M9" },
    ]);
    expect(index.query("SELECT kind, target, line FROM citations")).toEqual([{ kind: "rule", target: "x/y", line: 3 }]);
    expect(index.check()).toEqual([]);
    index.close();
  });

  test("a dangling reference, a schema issue, an orphan file and a failed invariant are each a finding", () => {
    const index = seed();
    write(goal, "stray", { milestone: "M404", live: false, tags: [], checks: [] }, tmp.root);
    write(chain, "chain", ["tools", "gone"], tmp.root);
    tmp.put("data/goals/bad.json", '{\n  "milestone": 3\n}\n');
    tmp.put("data/nobody/x.json", "{}\n");
    index.refresh();
    expect(index.check()).toEqual([
      { path: "data/chain.json", message: 'chain__items.value names "gone", which is no goal record' },
      { path: "data/goals/bad.json", message: "a goal has a check: bad has none" },
      { path: "data/goals/bad.json", message: "checks is missing" },
      { path: "data/goals/bad.json", message: "live is missing" },
      { path: "data/goals/bad.json", message: "milestone wants a string, found number 3" },
      { path: "data/goals/bad.json", message: "tags is missing" },
      { path: "data/goals/stray.json", message: "a goal has a check: stray has none" },
      { path: "data/goals/stray.json", message: 'goal.milestone names "M404", which is no milestone record' },
      { path: "data/nobody/x.json", message: "is under data/ but no record type claims it" },
    ]);
    index.close();
  });

  test("a rebuild reads only what changed, and forgets what was deleted", () => {
    const index = seed();
    index.refresh();
    expect(index.refresh()).toEqual({ read: 0, unchanged: 4, removed: 0 });
    // A touch without an edit is read, hashed and found unchanged.
    const future = new Date(Date.now() + 60_000);
    utimesSync(join(tmp.root, "docs/a.md"), future, future);
    expect(index.refresh()).toEqual({ read: 0, unchanged: 4, removed: 0 });
    write(goal, "tools", { milestone: "M9", live: false, tags: [], checks: [{ id: "one", stage: 1 }] }, tmp.root);
    expect(index.refresh()).toEqual({ read: 1, unchanged: 3, removed: 0 });
    expect(index.query("SELECT count(*) AS n FROM goal__tags")).toEqual([{ n: 0 }]);
    expect(index.query("SELECT id FROM goal__checks")).toEqual([{ id: "one" }]);
    rmSync(join(tmp.root, "docs/a.md"));
    expect(index.refresh()).toEqual({ read: 0, unchanged: 3, removed: 1 });
    expect(index.query("SELECT count(*) AS n FROM citations")).toEqual([{ n: 0 }]);
    index.close();
  });

  test("the index persists across opens, and a changed declaration rebuilds it", () => {
    const first = seed();
    first.refresh();
    first.close();
    const again = new Index({ root: tmp.root, types: TYPES, prose: ["docs/**/*.md"] });
    expect(again.refresh().read).toBe(0);
    again.close();
    const widened = defineRecord({ ...milestone, schema: s.object({ title: s.string(), estimate: s.optional(s.int()) }) });
    const rebuilt = new Index({ root: tmp.root, types: [widened, goal, chain], prose: ["docs/**/*.md"] });
    expect(rebuilt.refresh().read).toBe(4);
    rebuilt.close();
  });

  test("query refuses to write", () => {
    const index = seed();
    index.refresh();
    expect(() => index.query("DELETE FROM goal")).toThrow();
    expect(index.query("SELECT count(*) AS n FROM goal")).toEqual([{ n: 1 }]);
    index.close();
  });

  test("a reference to an undeclared type is refused when the index is built", () => {
    const loose = defineRecord({ name: "loose", dir: "loose", schema: s.object({ to: s.ref("nothing") }) });
    expect(() => new Index({ types: [loose], file: ":memory:" })).toThrow("nothing");
  });
});
