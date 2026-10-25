import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { hostname } from "node:os";
import { join } from "node:path";
import { dump, load, profile, stale, today, width } from "../lib/machine.ts";
import { scratch } from "./scratch.ts";

test("the width is half the cores, floor two, capped by the item count", () => {
  expect(width(16)).toBe(8);
  expect(width(4)).toBe(2);
  expect(width(2)).toBe(2);
  expect(width(1)).toBe(1);
  expect(width(0)).toBe(1);
  expect(width(16, { ceiling: 3 })).toBe(3);
});

test("an entry is stale on another host, a bad date or past thirty days", () => {
  const now = new Date(2026, 8, 25);
  const fresh = { cores: 4, host: hostname(), probed: "2026-09-01" };
  expect(stale(fresh, now)).toBe(false);
  expect(stale({ ...fresh, probed: "2026-08-26" }, now)).toBe(false);
  expect(stale({ ...fresh, probed: "2026-08-25" }, now)).toBe(true);
  expect(stale({ ...fresh, host: "elsewhere" }, now)).toBe(true);
  expect(stale({ ...fresh, probed: "soon" }, now)).toBe(true);
  expect(stale(undefined, now)).toBe(true);
});

test("a probe is cached, and a context nobody can reach keeps its entry", () => {
  const s = scratch();
  try {
    const cache = join(s.root, "machine.json");
    const first = profile("local", { cache, probe: () => ({ cores: 6 }) });
    expect(first).toEqual({ cores: 6, host: hostname(), probed: today() });
    expect(profile("local", { cache, probe: () => ({ cores: 99 }) }).cores).toBe(6);
    expect(profile("wsl", { cache })).toEqual({ cores: 1 });
    expect(profile("local", { cache, refresh: true, probe: () => ({}) }).cores).toBe(6);
    const doc = load(cache);
    expect(doc.contexts?.wsl).toBeUndefined();
    expect(readFileSync(cache, "utf8")).toBe(dump(doc));
    expect(dump({ version: 1, contexts: { b: { z: 1, a: 2 } } })).toBe(
      '{\n "contexts": {\n  "b": {\n   "a": 2,\n   "z": 1\n  }\n },\n "version": 1\n}',
    );
  } finally {
    s.cleanup();
  }
});
