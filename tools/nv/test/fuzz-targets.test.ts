// Each fuzz target is a wrapper around its check in `crates/nvs-fuzz-checks`, which `cargo test` replays
// over `fuzz/seeds/<target>/`. So a target whose file runs a check of its own would be fuzzed and never
// replayed. A target in `BY_HAND` is the one exception: it is a differential that runs only under
// `bun nv fuzz <target>`, and the list says why.

import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { BY_HAND, allTargets } from "../cmd/fuzz.ts";
import { ROOT } from "../lib/paths.ts";

const DIR = join(ROOT, "fuzz/fuzz_targets");

test("every fuzz target but the hand-run ones calls its check in nvs_fuzz_checks and nothing else", () => {
  const files = readdirSync(DIR);
  const targets = allTargets(files);
  expect(targets.length).toBeGreaterThan(0);
  expect(Object.keys(BY_HAND).every((name) => files.includes(`${name}.rs`))).toBe(true);
  for (const target of targets) {
    const code = readFileSync(join(DIR, `${target}.rs`), "utf8")
      .split("\n")
      .filter((line) => !line.trimStart().startsWith("//") && line.trim() !== "")
      .join("\n");
    expect(code).toContain(`nvs_fuzz_checks::${target}(data)`);
    expect(code.split("\n").length).toBeLessThanOrEqual(4);
  }
});
