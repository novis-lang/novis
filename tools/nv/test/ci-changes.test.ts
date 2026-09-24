import { expect, test } from "bun:test";
import { manifestGraph } from "../cmd/ci-changes.ts";
import { metadata } from "../keys/graph.ts";

// The `changes` job reads the graph off the manifests because it runs before cargo is installed, so
// this holds that reading to the one cargo gives wherever cargo is present.
test("the graph read off the manifests is the one cargo metadata gives", async () => {
  const cargo = await metadata();
  if (!cargo) return;
  const shape = (g: NonNullable<typeof cargo>) =>
    Object.fromEntries([...g].sort(([a], [b]) => (a < b ? -1 : 1)).map(([n, p]) => [n, { dir: p.dir, deps: Object.fromEntries([...p.deps].sort()) }]));
  expect(shape(manifestGraph()!)).toEqual(shape(cargo));
}, 120_000);
