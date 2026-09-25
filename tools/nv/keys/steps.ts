// What each `bun nv verify` step reads, and one key per step over exactly that. A step whose key has
// not moved since it was last green is answered from the green cache rather than run, so a key here is
// never narrower than what its step can observe.
//
// A step that compiles Rust keys on `builtFrom`, the one key of what a build is compiled from:
// `build` over every workspace package at `code`, `clippy` over every package with its tests at
// `docs`, and the doc-tests and `doc` at `docs`. A step that only runs `nvs` keys on what the CLI is
// compiled from, at `card` for the case trees and at `shipped` for `reference` and `extension`, which
// read the binary's text too. The script steps key on the files they read as bytes. `test` keys on
// every file but the tools' and the plan's, and each of its binaries has a narrower key of its own in
// `checks.ts`.
//
// With no `cargo metadata` graph, a step that compiles keys on every file in the tree.

import { type Graph } from "./graph.ts";
import { type Build, type Part, builtFrom, keyOf } from "./key.ts";
import { partitionOf } from "./partition.ts";
import { digest } from "./scan.ts";
import { type Tree, under } from "./tree.ts";

/** What `tools/owners.py` reads beside the doc comments under `crates/`. No other step reads these. */
export const OWNERS_READS = ["docs/implementation-plan.md", "docs/plan", "docs/agent/goals", "tools/owners.py", "tools/goals.py"];
/** What the `nv` step reads. No other step reads these. */
export const NV_READS = ["tools/nv", "data", "package.json", "bun.lock", "tsconfig.json"];

const MANIFEST_NAMES = new Set(["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]);

function isManifest(rel: string): boolean {
  const name = rel.slice(rel.lastIndexOf("/") + 1);
  return name === "config.toml" ? rel.endsWith(".cargo/config.toml") : MANIFEST_NAMES.has(name);
}

const isRust = (rel: string) => rel.endsWith(".rs");
const within = (...tops: string[]) => (rel: string) => tops.some((t) => under(rel, t));
const only = (...names: string[]) => (rel: string) => names.includes(rel);

/** Every file `pred` takes, as bytes. */
function files(tree: Tree, pred: (rel: string) => boolean): Part[] {
  return tree.files.filter(pred).map((rel) => ({ label: rel, partition: partitionOf(rel), tier: "raw", digest: tree.raw(rel) }));
}

function toolchain(tree: Tree): Part[] {
  return [{ label: "<rustc>", partition: "toolchain", tier: "raw", digest: digest(tree.toolchain) }];
}

function compiled(tree: Tree, graph: Graph | null, build: Omit<Build, "own"> & { own?: string[] }): Part[] {
  if (graph === null) return [...toolchain(tree), ...files(tree, () => true)];
  return builtFrom(tree, graph, { ...build, own: build.own ?? [...graph.keys()].sort() });
}

const cli = (tier: "card" | "shipped"): Omit<Build, "own"> & { own: string[] } => ({ own: ["nvs-cli"], ownTier: tier, depTier: tier, test: false });

/** Each step's parts. A step that is absent is never answered from the cache. */
export const STEP_READS: Record<string, (tree: Tree, graph: Graph | null) => Part[]> = {
  fmt: (t) => files(t, (r) => isRust(r) || isManifest(r) || r === "rustfmt.toml"),
  lints: (t) => files(t, (r) => isManifest(r) || r === "tools/nv/cmd/lints.ts"),
  directives: (t) => files(t, (r) => within("crates", "benches")(r) || r === "tools/nv/cmd/directives.ts"),
  template: (t) => files(t, (r) => within("crates", "benches")(r) || r === "tools/nv/cmd/directives.ts"),
  owners: (t) => files(t, within("crates", ...OWNERS_READS)),
  nv: (t) => files(t, within(...NV_READS)),
  "fuzz-lock": (t) => [...toolchain(t), ...files(t, isManifest)],
  build: (t, g) => compiled(t, g, { ownTier: "code", depTier: "code", test: false }),
  test: (t) => [...toolchain(t), ...files(t, (r) => !within(...OWNERS_READS, ...NV_READS)(r))],
  // The one test job that reads no file as text: a doc-test is a doc comment, compiled.
  "test:doc": (t, g) => compiled(t, g, { ownTier: "docs", depTier: "docs", test: false }),
  conformance: (t, g) => [...compiled(t, g, cli("card")), ...files(t, within("tests/conformance"))],
  differential: (t, g) => [...compiled(t, g, cli("card")), ...files(t, within("tests/differential"))],
  reference: (t, g) => [...compiled(t, g, cli("shipped")), ...files(t, (r) => within("docs/reference")(r) || only("tools/reference.py")(r))],
  clippy: (t, g) => compiled(t, g, { ownTier: "docs", depTier: "docs", test: true }),
  extension: (t, g) => [...compiled(t, g, cli("shipped")), ...files(t, within("editors"))],
  doc: (t, g) => compiled(t, g, { ownTier: "docs", depTier: "docs", test: false }),
};

/** The key `step` is green under, scoped by `-p` for the `test` step, or `null` for a step this table
 * does not describe, which `verify` takes as "always run". */
export function stepKey(tree: Tree, graph: Graph | null, step: string, scope?: string): string | null {
  const reads = STEP_READS[step];
  if (reads === undefined) return null;
  return keyOf(`${step}\0${scope ?? ""}`, reads(tree, graph));
}
