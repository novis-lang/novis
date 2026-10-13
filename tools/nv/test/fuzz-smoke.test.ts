// The CI job that fuzzes every target: its matrix names each file of `fuzz/fuzz_targets/`, so a target
// added there is fuzzed in the deep lane without anyone remembering to list it.

import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

test("the fuzz-smoke matrix lists every fuzz target, and nothing else", () => {
  const yml = Bun.YAML.parse(readFileSync(join(ROOT, ".github/workflows/ci.yml"), "utf8")) as {
    jobs: Record<string, { strategy?: { matrix?: { target?: string[] } } }>;
  };
  const listed = [...(yml.jobs["fuzz-smoke"]?.strategy?.matrix?.target ?? [])].sort();
  const targets = readdirSync(join(ROOT, "fuzz/fuzz_targets"))
    .filter((name) => name.endsWith(".rs"))
    .map((name) => name.slice(0, -".rs".length))
    .sort();
  expect(targets.length).toBeGreaterThan(0);
  expect(listed).toEqual(targets);
});
