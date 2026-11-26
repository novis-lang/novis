import { describe, expect, test } from "bun:test";
import type { TraceEvent } from "../src/api.ts";
import { cacheGroups, outcomeText } from "../src/views/request/outcome.ts";

// An event's outcome is drawn as short text after its label, and the Cache tab counts a key's hits
// and misses from it.

const event = (over: Partial<TraceEvent>): TraceEvent => ({
  id: 1,
  kind: "cache",
  label: "Core\\Cache\\Store::get",
  start_us: 0,
  end_us: 10,
  status: null,
  query: null,
  ...over,
});

describe("outcomeText", () => {
  test("each kind's outcome is short text, and an event without one has none", () => {
    expect(outcomeText(event({ kind: "http", outcome: { http_status: 503, bytes_in: 17, bytes_out: 4 } }))).toBe("503 · 17 B in · 4 B out");
    expect(outcomeText(event({ kind: "file", outcome: { bytes: 2048 } }))).toBe("2.00 KiB");
    expect(outcomeText(event({ kind: "spawn", outcome: { command: "convert", exit: 2 } }))).toBe("convert · exit code 2");
    expect(outcomeText(event({ outcome: { hit: true, tier: "redis" } }))).toBe("shared store");
    expect(outcomeText(event({ kind: "session" }))).toBe("");
  });
});

describe("cacheGroups", () => {
  test("a key's miss and then its hit are one group with one of each", () => {
    const events = [
      event({ id: 1, target: "item:7", outcome: { hit: false, tier: "memory" } }),
      event({ id: 2, label: "Core\\Cache\\Store::put", target: "item:7", outcome: { tier: "memory" } }),
      event({ id: 3, target: "item:7", outcome: { hit: true, tier: "memory" } }),
      event({ id: 4, label: "Core\\Cache\\Store::getSecret", target: null, outcome: { tier: "memory" } }),
      event({ id: 5, kind: "query", label: "sqlite" }),
    ];
    const groups = cacheGroups({ events });
    expect(groups.map((g) => [g.key, g.tier, g.count, g.hits, g.misses])).toEqual([
      ["item:7", "memory", 3, 1, 1],
      [null, "memory", 1, 0, 0],
    ]);
  });
});
