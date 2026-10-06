import { describe, expect, test } from "bun:test";
import { type Reach, benchOrder, planBenches } from "../proofs/select.ts";

/** Runs `planBenches` with every bench in `bad` showing a degradation. */
function plan(reach: Reach[], bad: string[] = []) {
  return planBenches(reach, async (path) => bad.includes(path));
}

describe("planBenches", () => {
  test("a bench whose changed items are all proven is skipped and its figure carried", async () => {
    const got = await plan([
      { path: "a", items: ["x", "y"] },
      { path: "b", items: ["x", "y"] },
      { path: "c", items: ["x"] },
      { path: "d", items: ["y"] },
    ]);
    expect(got.ran).toEqual(["a", "b"]);
    expect([...got.skipped]).toEqual([
      ["c", ["a", "b"]],
      ["d", ["a", "b"]],
    ]);
    expect(got.degraded).toEqual([]);
  });

  test("an item fewer than two benches reach runs every bench that reaches it", async () => {
    const got = await plan([
      { path: "a", items: ["x", "y"] },
      { path: "b", items: ["x"] },
      { path: "c", items: ["x", "z"] },
      { path: "new", items: null },
    ]);
    // `y` and `z` are each reached by one bench, and a bench with no recorded footprint always runs.
    expect(got.ran).toEqual(["new", "a", "c"]);
    expect([...got.skipped]).toEqual([["b", ["a", "c"]]]);
  });

  test("a degradation runs every bench that reaches the same changed items", async () => {
    const reach: Reach[] = [
      { path: "a", items: ["x", "y", "z"] },
      { path: "b", items: ["x", "y"] },
      { path: "c", items: ["x", "y"] },
      { path: "d", items: ["x", "z"] },
      { path: "e", items: ["y"] },
    ];
    // `c` is passed over once `a` and `b` prove `x` and `y`. Then `d` runs for `z` and degrades, so `c`,
    // which shares `x` with it, runs after all.
    const got = await plan(reach, ["d"]);
    expect(got.degraded).toEqual(["d"]);
    expect(got.ran).toEqual(["a", "b", "d", "c"]);
    expect([...got.skipped]).toEqual([["e", ["a", "b"]]]);
  });

  test("the same change always picks the same benches", async () => {
    const reach: Reach[] = [
      { path: "m", items: ["x"] },
      { path: "k", items: ["x", "y"] },
      { path: "z", items: ["x"] },
      { path: "a", items: ["x"] },
      { path: "q", items: ["y", "x"] },
    ];
    expect(benchOrder(reach).map((r) => r.path)).toEqual(["k", "q", "a", "m", "z"]);
    const once = await plan(reach);
    const again = await plan([...reach].reverse());
    expect(again).toEqual(once);
    expect(once.ran).toEqual(["k", "q"]);
  });
});
