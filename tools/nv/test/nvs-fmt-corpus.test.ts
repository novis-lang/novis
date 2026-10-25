import { describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { held, outside, parseCorpus, readCorpus } from "../lib/nvs-fmt-corpus.ts";
import { ROOT } from "../lib/paths.ts";

describe("parseCorpus", () => {
  test("reads trees and skips, and leaves comments and blank lines out", () => {
    const c = parseCorpus("# a comment\n\ntree docs/examples\ntree tests/\nskip tests/fmt/input  --  wrong on purpose\n");
    expect(c).toEqual({ trees: ["docs/examples", "tests"], skips: ["tests/fmt/input"] });
  });

  test("a skip without its reason, and a line of another kind, are errors", () => {
    expect(() => parseCorpus("skip tests/fmt/input\n")).toThrow(/a skip says why/);
    expect(() => parseCorpus("take tests\n")).toThrow(/expected `tree <dir>`/);
  });
});

describe("held and outside", () => {
  const c = parseCorpus("tree tests\ntree docs/examples\nskip tests/fmt/input  --  why\n");

  test("a file under a tree is held, and one under its skip is not", () => {
    expect(held(c, "tests/hostile/a.nvs")).toBe(true);
    expect(held(c, "tests/fmt/input/a.nvs")).toBe(false);
    expect(outside(c, "tests/fmt/input/a.nvs")).toBe(false);
  });

  test("a skip can name one file", () => {
    const one = parseCorpus("tree tests\nskip tests/check/one-line.nvs  --  why\n");
    expect(held(one, "tests/check/one-line.nvs")).toBe(false);
    expect(held(one, "tests/check/one-line.nvs.bak/a.nvs")).toBe(true);
  });

  test("a directory that only starts with a tree's name is outside it", () => {
    expect(held(c, "docs/examples-old/a.nvs")).toBe(false);
    expect(outside(c, "docs/examples-old/a.nvs")).toBe(true);
  });
});

test("every tree and skip of the tree's own corpus file exists", () => {
  const c = readCorpus();
  for (const dir of [...c.trees, ...c.skips.filter((s) => !s.endsWith("/generated"))]) expect(existsSync(join(ROOT, dir))).toBe(true);
});
