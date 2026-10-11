import { afterEach, beforeEach, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { SIDE_ENV } from "../lib/chain.ts";
import { apply, PLAN, planFields } from "../cmd/plan.ts";
import { type Scratch, scratch } from "./scratch.ts";

// A plan whose fields aim at 10 bytes and may not grow past 2x that, one milestone with its scope file.
const STATUS = { status: "s", done: "d", onDisk: "o", toolchain: "t",openNow: "the old sentence", blocking: "b" };
const PLAN_MD = [
  "# Plan",
  "",
  "<!-- Aim for ~10 bytes a field. A wrap that leaves a field over 2x that and bigger than it was is refused. -->",
  "",
  "> **Status:** s",
  "> **Done:** d",
  "> **On disk:** o",
  "> **Toolchain:** t",
  "> **Open now:** the old sentence",
  "> **Blocking:** b",
  "",
  "The rest of the plan.",
  "",
].join("\n");
const MILESTONE_MD = "# M1 — First\n\nThe old scope.\n\n**Verify:** it runs.\n";

let s: Scratch;
let side: string | undefined;

beforeEach(() => {
  s = scratch();
  s.put(PLAN, PLAN_MD);
  s.put("data/plan/status.json", `${JSON.stringify(STATUS, null, 2)}\n`);
  s.put("data/plan/milestones/M1.json", `${JSON.stringify({ title: "First", order: 1, loopDays: "1", state: "open" }, null, 2)}\n`);
  s.put("docs/plan/m1.md", MILESTONE_MD);
  side = process.env[SIDE_ENV];
  delete process.env[SIDE_ENV];
});

afterEach(() => {
  if (side === undefined) delete process.env[SIDE_ENV];
  else process.env[SIDE_ENV] = side;
  s.cleanup();
});

const read = (path: string) => readFileSync(join(s.root, path), "utf8");

const WRAP = [
  "## plan: done",
  "D2",
  "",
  "## plan-edit: Open now",
  "--- old",
  "old",
  "--- new",
  "new",
  "",
  "## milestone: m1",
  "The new scope.",
  "",
  "**Verify:** it still runs.",
  "",
  "## status",
  "CONTINUE ignored here",
  "",
].join("\n");

test("nv plan --apply --dry checks the plan sections and writes nothing", () => {
  const { code, lines } = apply(WRAP, true, s.root);
  expect(code).toBe(0);
  expect(lines[0]).toBe("nv plan --apply: would apply 3 section(s)");
  expect(read(PLAN)).toBe(PLAN_MD);
  expect(read("docs/plan/m1.md")).toBe(MILESTONE_MD);
  expect(planFields(s.root).find(([n]) => n === "Open now")![1]).toBe("the old sentence");

  // A dry run still checks: a fragment the field does not hold is refused.
  const bad = apply("## plan-edit: Open now\n--- old\nmissing\n--- new\nx\n", true, s.root);
  expect(bad.code).toBe(1);
  expect(bad.lines.join("\n")).toContain("is not in that field");
});

test("nv plan --apply writes a plan field, a plan edit and a milestone", () => {
  const { code, lines } = apply(WRAP, false, s.root);
  expect(code).toBe(0);
  expect(lines).toHaveLength(4);
  const plan = read(PLAN);
  expect(plan).toContain("> **Done:** D2\n");
  expect(plan).toContain("> **Open now:** the new sentence\n");
  expect(plan).toContain("The rest of the plan.");
  const fields = new Map(planFields(s.root));
  expect(fields.get("Done")).toBe("D2");
  expect(fields.get("Open now")).toBe("the new sentence");
  expect(read("docs/plan/m1.md")).toBe("# M1 — First\n\nThe new scope.\n\n**Verify:** it still runs.\n");
});

test("nv plan --apply refuses a plan field that grows past its ceiling", () => {
  const { code, lines } = apply("## plan: Blocking\nthis text is far longer than twenty bytes\n", false, s.root);
  expect(code).toBe(1);
  expect(lines[0]).toBe("nv plan --apply: NOTHING was written:");
  expect(lines.join("\n")).toContain("past its 20 B ceiling");
  expect(read(PLAN)).toBe(PLAN_MD);

  // An edit that leaves an over-ceiling field no bigger is taken.
  const shrink = apply("## plan: Blocking\nB\n", false, s.root);
  expect(shrink.code).toBe(0);
});
