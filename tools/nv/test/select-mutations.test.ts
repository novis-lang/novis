import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { planEdits, readMutations } from "../select/mutate.ts";

describe("tools/data/select-mutations.json", () => {
  test("it reads, every edit's find occurs once in the tree, and no two edits of a batch touch one file", () => {
    const doc = readMutations(join(ROOT, "tools", "data", "select-mutations.json"));
    expect(doc.batches.length).toBeGreaterThanOrEqual(3);
    for (const batch of doc.batches) {
      expect(new Set(batch.map((e) => e.file)).size).toBe(batch.length);
      expect(planEdits(batch, ROOT).length).toBe(batch.length);
    }
  });
});
