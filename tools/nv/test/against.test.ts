import { expect, test } from "bun:test";
import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";
import { CORPUS, compare, firstDifference, parse, record, report } from "../cmd/against.ts";
import { ROOT } from "../lib/paths.ts";
import { releaseCli } from "../lib/relink.ts";

test("against compares by default, records with --record, and takes rounds and folders", () => {
  expect(parse([])).toEqual({ record: false, rounds: 5, corpus: CORPUS });
  expect(parse(["--record"])).toEqual({ record: true, rounds: 5, corpus: CORPUS });
  expect(parse(["--rounds", "9", "--corpus", "a", "--corpus", "b"])).toEqual({ record: false, rounds: 9, corpus: ["a", "b"] });
  expect(parse(["--rounds", "0"])).toBe("--rounds needs a whole number of 1 or more, not 0");
  expect(parse(["--corpus"])).toBe("--corpus needs a folder");
  expect(parse(["--fast"])).toBe("unknown argument --fast");
});

test("the first difference names its line and both sides", () => {
  expect(firstDifference("a\nb\nc", "a\nx\nc")).toBe("line 2: b | x");
  expect(firstDifference("a", "a\nb")).toBe("line 2: (end) | b");
  expect(firstDifference("a", "a")).toBe("no line differs");
});

// A record of the current binary, then a compare of that same binary against it, on a small corpus. The
// work folder is the test's own, so a base `bun nv against --record` wrote stays as it was.
test.skipIf(!existsSync(releaseCli()))(
  "a build compared with a record of itself has the same IR, the same diagnostics and a ratio near 1",
  async () => {
    const work = join(ROOT, ".agent-tmp", "against-test");
    const corpus = ["docs/examples/core/Taint"];
    try {
      expect(await record(work, corpus)).toBe(0);
      const c = await compare(work, corpus, 5);
      if (typeof c === "string") throw new Error(c);
      const printed = report(c, 5).join("\n");
      expect(c.irDiffers, printed).toEqual([]);
      expect(c.diagDiffers, printed).toEqual([]);
      expect(c.added).toEqual([]);
      expect(c.timings.length).toBe(6);
      expect(c.ratio, printed).toBeGreaterThan(0.9);
      expect(c.ratio, printed).toBeLessThan(1.1);
    } finally {
      rmSync(work, { recursive: true, force: true });
    }
  },
  300_000,
);
