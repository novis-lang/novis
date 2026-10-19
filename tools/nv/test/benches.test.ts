// `bun nv benches` with its bench turn handed in, so nothing builds and no bench runs.

import { describe, expect, test } from "bun:test";
import { benches } from "../cmd/benches.ts";


/** Runs `bun nv benches args` over a turn that records the `owedOnly` it was handed and returns `red`. */
async function benchesOver(args: string[], red = "") {
  const turns: boolean[] = [];
  const code = await benches(args, async (_note, owedOnly = false) => {
    turns.push(owedOnly);
    return red;
  });
  return { code, turns };
}

describe("bun nv benches", () => {
  test("nv benches takes --owed and no other flag", async () => {
    expect(await benchesOver([])).toEqual({ code: 0, turns: [false] });
    expect(await benchesOver(["--owed"])).toEqual({ code: 0, turns: [true] });
    expect(await benchesOver(["--owed"], "benches: 1 red bench(es), their growth failed: a.nvs")).toEqual({ code: 1, turns: [true] });
    for (const args of [["--all"], ["--owed", "--owed"], ["--owed", "x"], ["owed"]]) {
      expect(await benchesOver(args)).toEqual({ code: 2, turns: [] });
    }
  });
});
