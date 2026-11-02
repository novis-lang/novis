import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { posix } from "node:path";
import { LANES, lanesFor, manifestGraph, releaseNeeded, UNBUILT } from "../cmd/ci-changes.ts";
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

/** Every path, from the repository's root, that a file matching `glob` names through `pattern`'s first
 * group, resolved against that file's directory. */
function named(glob: string, pattern: RegExp): string[] {
  const out: string[] = [];
  for (const file of new Bun.Glob(glob).scanSync({ cwd: ROOT })) {
    const rel = file.replaceAll("\\", "/");
    for (const m of readFileSync(posix.join(ROOT, rel), "utf8").matchAll(pattern)) {
      out.push(posix.normalize(posix.join(posix.dirname(rel), m[1]!)));
    }
  }
  return out;
}

const embedded = () => named("crates/**/*.rs", /include_(?:str|bytes)!\(\s*"([^"]+)"/g);

// A path crate or an embedded file outside the `code` lane would let a push change the binary and
// run none of its tests.
test("the code lane covers every file a crate embeds and every path crate", () => {
  const pathCrates = ["Cargo.toml", "crates/*/Cargo.toml"].flatMap((glob) => named(glob, /\bpath\s*=\s*"([^"]+)"/g));
  const inputs = [...embedded(), ...pathCrates.map((dir) => `${dir}/Cargo.toml`)];
  expect(pathCrates.some((dir) => dir.startsWith("vendor/"))).toBe(true);
  expect(inputs.filter((path) => !lanesFor({ code: LANES.code! }, [path])[0]![1])).toEqual([]);
});

// A file the binary embeds under a path `UNBUILT` names would leave the dev build stale after a push
// that changed it.
test("no file a crate embeds is on the list the dev build skips", () => {
  const files = embedded();
  expect(files.length).toBeGreaterThan(0);
  expect(files.filter((path) => !releaseNeeded([path]))).toEqual([]);
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
