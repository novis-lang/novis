// The CI job that fuzzes every target: its legs together name each file of `fuzz/fuzz_targets/` once,
// so a target added there is fuzzed in the deep lane without anyone remembering to list it. A target in
// `BY_HAND` is the one exception: it runs only under `bun nv fuzz <target>`, and the list says why.

import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

/** The targets CI does not fuzz, each with its reason. */
const BY_HAND: Record<string, string> = {
  // A differential against comrak. Its open differences are cmark-gfm edge cases with no security
  // value, so in CI they would turn the deep run red at random (`rule:core-classes/markdown-cost`).
  markdown_comrak: "known differences from cmark-gfm on unusual input",
};

test("the fuzz-smoke legs list every fuzz target once, but the hand-run ones, and nothing else", () => {
  const yml = Bun.YAML.parse(readFileSync(join(ROOT, ".github/workflows/ci.yml"), "utf8")) as {
    jobs: Record<string, { strategy?: { matrix?: { include?: { targets?: string[] }[] } } }>;
  };
  const legs = yml.jobs["fuzz-smoke"]?.strategy?.matrix?.include ?? [];
  const listed = legs.flatMap((leg) => leg.targets ?? []).sort();
  const targets = readdirSync(join(ROOT, "fuzz/fuzz_targets"))
    .filter((name) => name.endsWith(".rs"))
    .map((name) => name.slice(0, -".rs".length))
    .sort();
  expect(targets.length).toBeGreaterThan(0);
  expect(Object.keys(BY_HAND).every((name) => targets.includes(name))).toBe(true);
  expect(listed).toEqual(targets.filter((name) => !(name in BY_HAND)));
});
