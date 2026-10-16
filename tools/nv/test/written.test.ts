import { afterEach, beforeEach, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { ENV, record } from "../lib/written.ts";
import { type Scratch, scratch } from "./scratch.ts";

let s: Scratch;
const saved = process.env[ENV];

beforeEach(() => {
  s = scratch();
  delete process.env[ENV];
});

afterEach(() => {
  if (saved === undefined) delete process.env[ENV];
  else process.env[ENV] = saved;
  s.cleanup();
});

test("record appends to the file LOOPER_WRITTEN names", () => {
  const looper = join(s.root, "looper.txt");
  process.env[ENV] = looper;
  record(join(ROOT, "docs", "a.md"));
  record(join(ROOT, "docs", "b.md"), join(ROOT, "tools", "c.ts"));
  expect(readFileSync(looper, "utf8")).toBe("docs/a.md\ndocs/b.md\ntools/c.ts\n");
});

test("record writes nothing when the variable is not set", () => {
  record(join(ROOT, "docs", "a.md"));
  expect(existsSync(join(s.root, "looper.txt"))).toBe(false);
});
