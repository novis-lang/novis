// `newestNvs`: the binary a check reads when none is named is the most recently written build, so a
// release binary from before the goal's changes is not read in place of the goal's own debug build.

import { afterAll, describe, expect, test } from "bun:test";
import { mkdirSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { newestNvs } from "../lib/nvs-binary.ts";
import { ROOT } from "../lib/paths.ts";

const TMP = join(ROOT, ".agent-tmp", `nvs-binary-test-${process.pid}`);
afterAll(() => rmSync(TMP, { recursive: true, force: true }));

/** Writes `path` under `root` with its modification time `secs` seconds after the epoch. */
function built(root: string, path: string, secs: number): void {
  const p = join(root, path);
  mkdirSync(dirname(p), { recursive: true });
  writeFileSync(p, "");
  utimesSync(p, secs, secs);
}

describe("newestNvs", () => {
  const candidates = ["target/debug/nvs", "target/release/nvs", "target/proof/nvs"];

  test("the most recently written build wins over an older release build", () => {
    const root = join(TMP, "newest");
    built(root, "target/release/nvs", 1_000);
    built(root, "target/debug/nvs", 2_000);
    expect(newestNvs(root, candidates)).toBe(join(root, "target/debug/nvs"));
  });

  test("a release build written after the debug build wins", () => {
    const root = join(TMP, "release");
    built(root, "target/debug/nvs", 1_000);
    built(root, "target/release/nvs", 2_000);
    expect(newestNvs(root, candidates)).toBe(join(root, "target/release/nvs"));
  });

  test("a tie goes to the earlier candidate, and no build at all is null", () => {
    const root = join(TMP, "tie");
    built(root, "target/proof/nvs", 1_000);
    built(root, "target/release/nvs", 1_000);
    expect(newestNvs(root, candidates)).toBe(join(root, "target/release/nvs"));
    expect(newestNvs(join(TMP, "none"), candidates)).toBeNull();
  });
});
