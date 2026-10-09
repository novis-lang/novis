import { afterEach, beforeEach, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { ENV, LOOPER_ENV, record } from "../lib/written.ts";
import { type Scratch, scratch } from "./scratch.ts";

let s: Scratch;
const saved = { [ENV]: process.env[ENV], [LOOPER_ENV]: process.env[LOOPER_ENV] };

beforeEach(() => {
  s = scratch();
  delete process.env[ENV];
  delete process.env[LOOPER_ENV];
});

afterEach(() => {
  for (const [k, v] of Object.entries(saved)) {
    if (v === undefined) delete process.env[k];
    else process.env[k] = v;
  }
  s.cleanup();
});

test("record appends to the files NOVIS_LOOP_WRITES and LOOPER_WRITTEN name", () => {
  const novis = join(s.root, "novis.txt");
  const looper = join(s.root, "looper.txt");
  process.env[ENV] = novis;
  process.env[LOOPER_ENV] = looper;
  record(join(ROOT, "docs", "a.md"));
  record(join(ROOT, "docs", "b.md"), join(ROOT, "tools", "c.ts"));
  const want = "docs/a.md\ndocs/b.md\ntools/c.ts\n";
  expect(readFileSync(novis, "utf8")).toBe(want);
  expect(readFileSync(looper, "utf8")).toBe(want);

  // Only the one that is set is written.
  delete process.env[ENV];
  record(join(ROOT, "docs", "d.md"));
  expect(readFileSync(novis, "utf8")).toBe(want);
  expect(readFileSync(looper, "utf8")).toBe(`${want}docs/d.md\n`);
});

test("record writes nothing when neither variable is set", () => {
  record(join(ROOT, "docs", "a.md"));
  expect(existsSync(join(s.root, "novis.txt"))).toBe(false);
  expect(existsSync(join(s.root, "looper.txt"))).toBe(false);
});
