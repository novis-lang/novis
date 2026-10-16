import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { hostname } from "node:os";
import { join } from "node:path";
import { dump, load, posixProbe, probeSh, profile, readProbe, remember, stale, today, width } from "../lib/machine.ts";
import { scratch } from "./scratch.ts";

test("the width is half the cores, floor two, capped by the item count and by free memory", () => {
  expect(width(16)).toBe(8);
  expect(width(4)).toBe(2);
  expect(width(2)).toBe(2);
  expect(width(1)).toBe(1);
  expect(width(0)).toBe(1);
  expect(width(16, { ceiling: 3 })).toBe(3);
  expect(width(16, { memKb: 1_000_000, workerKb: 200_000 })).toBe(2);
  expect(width(16, { memKb: 1_000_000 })).toBe(8);
});

test("the probe reader takes the first of each known field and ignores the rest", () => {
  expect(readProbe("cores 16\nmem_kb 15000 kB\nsample_s 3.21 sample_rss_kb 55120\ncores 2\nnoise 7")).toEqual({
    cores: 16,
    mem_kb: 15000,
    sample_s: 3.21,
    sample_rss_kb: 55120,
  });
  expect(readProbe("")).toEqual({});
  expect(probeSh("it's")).toBe(probeSh());
  expect(probeSh("true")).toContain("sh -c 'true'");
});

test("a failed sample drops its timing and leaves the entry pending", () => {
  const got = posixProbe(() => ({ code: 0, text: "cores 8\nsample_code 1\nsample_s 2.5 sample_rss_kb 100" }), "false");
  expect(got).toEqual({ cores: 8, sample_pending: 1 });
  expect(posixProbe(() => ({ code: 1, text: "" }))).toEqual({});
});

test("an entry is stale on another host, a pending sample, a bad date or past thirty days", () => {
  const now = new Date(2026, 8, 25);
  const fresh = { cores: 4, host: hostname(), probed: "2026-09-01" };
  expect(stale(fresh, now)).toBe(false);
  expect(stale({ ...fresh, probed: "2026-08-26" }, now)).toBe(false);
  expect(stale({ ...fresh, probed: "2026-08-25" }, now)).toBe(true);
  expect(stale({ ...fresh, host: "elsewhere" }, now)).toBe(true);
  expect(stale({ ...fresh, sample_pending: 1 }, now)).toBe(true);
  expect(stale({ ...fresh, probed: "soon" }, now)).toBe(true);
  expect(stale(undefined, now)).toBe(true);
});

test("a probe is cached, a context nobody can reach keeps its entry, and remembered keys survive", () => {
  const s = scratch();
  try {
    const cache = join(s.root, "machine.json");
    const first = profile("local", { cache, probe: () => ({ cores: 6 }) });
    expect(first).toEqual({ cores: 6, host: hostname(), probed: today() });
    expect(profile("local", { cache, probe: () => ({ cores: 99 }) }).cores).toBe(6);
    expect(profile("wsl", { cache })).toEqual({ cores: 1 });
    expect(profile("local", { cache, refresh: true, probe: () => ({}) }).cores).toBe(6);
    remember("local", { sweep_s: 12.5, gone: null }, cache);
    remember("wsl", { sweep_s: 1 }, cache);
    const doc = load(cache);
    expect(doc.contexts?.local?.sweep_s).toBe(12.5);
    expect(doc.contexts?.wsl).toBeUndefined();
    expect(readFileSync(cache, "utf8")).toBe(dump(doc));
    expect(dump({ version: 1, contexts: { b: { z: 1, a: 2 } } })).toBe(
      '{\n "contexts": {\n  "b": {\n   "a": 2,\n   "z": 1\n  }\n },\n "version": 1\n}',
    );
  } finally {
    s.cleanup();
  }
});
