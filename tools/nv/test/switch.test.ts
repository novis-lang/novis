import { afterEach, expect, test } from "bun:test";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { type Goal, goalFiles, leaveGoal } from "../lib/chain.ts";
import { load, write } from "../lib/store.ts";
import { chain } from "../schema/chain.ts";
import { goal } from "../schema/goal.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch | undefined;
afterEach(() => {
  tmp?.cleanup();
  tmp = undefined;
});

const record = (id: string): Goal => ({
  title: "t",
  milestone: null,
  files: [],
  context: {},
  stages: [{ number: 1, title: "x", summary: "s" }],
  checks: [{ id, stage: 1, kind: "command", argv: [id] }],
  env: {},
});

/** A chain of `a`, `b` and `c` with `a` live, each goal with its prose and record, and `a` with a handoff. */
function seed(): Scratch {
  const s = scratch();
  write(chain, "chain", { live: "a", goals: ["a", "b", "c"] }, s.root);
  mkdirSync(join(s.root, "docs/agent/goals"), { recursive: true });
  for (const slug of ["a", "b", "c"]) {
    write(goal, slug, record(`t${slug}`), s.root);
    writeFileSync(join(s.root, `docs/agent/goals/${slug}.md`), `# ${slug}\n`);
  }
  writeFileSync(join(s.root, "data/goals/a.handoff.json"), "{}\n");
  return s;
}

test("the switch deletes the goal it leaves, takes it off the chain and makes the next goal live", () => {
  tmp = seed();
  const changed = leaveGoal("a", "b", tmp.root);
  expect(changed.sort()).toEqual(["data/chain.json", ...goalFiles("a")].sort());
  for (const p of goalFiles("a")) expect(existsSync(join(tmp.root, p))).toBe(false);
  expect(load(chain, tmp.root)[0]!.value).toEqual({ live: "b", goals: ["b", "c"] });
  expect(existsSync(join(tmp.root, "docs/agent/goals/b.md"))).toBe(true);
  expect(load(goal, tmp.root).map((g) => g.id).sort()).toEqual(["b", "c"]);
});

test("a goal file that is already gone is not reported, and the rest still go", () => {
  tmp = seed();
  expect(leaveGoal("b", "c", tmp.root).sort()).toEqual(["data/chain.json", "data/goals/b.json", "docs/agent/goals/b.md"]);
  expect(load(chain, tmp.root)[0]!.value).toEqual({ live: "c", goals: ["a", "c"] });
});

test("a goal that is not on the chain changes nothing", () => {
  tmp = seed();
  expect(() => leaveGoal("a", "gone", tmp!.root)).toThrow("not on the chain");
  expect(existsSync(join(tmp.root, "docs/agent/goals/a.md"))).toBe(true);
  expect(load(chain, tmp.root)[0]!.value).toEqual({ live: "a", goals: ["a", "b", "c"] });
});
