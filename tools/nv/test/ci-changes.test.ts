import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { posix } from "node:path";
import { manifestGraph, releaseNeeded, UNBUILT } from "../cmd/ci-changes.ts";
import { metadata } from "../keys/graph.ts";
import { ROOT } from "../lib/paths.ts";

test("a diff of docs alone needs no dev build, and anything else or an unknown diff does", () => {
  expect(releaseNeeded(["docs/goals/release-run.md", "AGENTS.md", "website/src/pages/index.astro"])).toBe(false);
  expect(releaseNeeded(["docs/goals/release-run.md", "crates/nvs-cli/src/main.rs"])).toBe(true);
  expect(releaseNeeded(["docs/reference/core/Str.md"])).toBe(true);
  expect(releaseNeeded(["CHANGELOG.md"])).toBe(true);
  expect(releaseNeeded(["a-directory-added-later/x.txt"])).toBe(true);
  expect(releaseNeeded([])).toBe(false);
  expect(releaseNeeded(null)).toBe(true);
});

// A file the binary embeds under a path `UNBUILT` names would leave the dev build stale after a push
// that changed it.
test("no file a crate embeds is on the list the dev build skips", () => {
  const embedded: string[] = [];
  for (const file of new Bun.Glob("crates/**/*.rs").scanSync({ cwd: ROOT })) {
    const rel = file.replaceAll("\\", "/");
    const text = readFileSync(posix.join(ROOT, rel), "utf8");
    for (const m of text.matchAll(/include_(?:str|bytes)!\(\s*"([^"]+)"/g)) {
      embedded.push(posix.normalize(posix.join(posix.dirname(rel), m[1]!)));
    }
  }
  expect(embedded.length).toBeGreaterThan(0);
  expect(embedded.filter((path) => !releaseNeeded([path]))).toEqual([]);
  expect(UNBUILT.filter((p) => p.startsWith("crates/") || p === "Cargo.toml" || p === "Cargo.lock")).toEqual([]);
});

// The `changes` job reads the graph off the manifests because it runs before cargo is installed, so
// this holds that reading to the one cargo gives wherever cargo is present.
test("the graph read off the manifests is the one cargo metadata gives", async () => {
  const cargo = await metadata();
  if (!cargo) return;
  const shape = (g: NonNullable<typeof cargo>) =>
    Object.fromEntries([...g].sort(([a], [b]) => (a < b ? -1 : 1)).map(([n, p]) => [n, { dir: p.dir, deps: Object.fromEntries([...p.deps].sort()) }]));
  expect(shape(manifestGraph()!)).toEqual(shape(cargo));
}, 120_000);
