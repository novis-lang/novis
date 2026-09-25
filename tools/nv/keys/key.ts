// The one key of what a Rust build is compiled from. `verify`, `impact`, `loop` and `proofs` call
// `builtFrom`, and no second copy of the rule exists:
//
// - the toolchain, and the workspace's manifests and lock;
// - the packages it compiles as its own, each file at `ownTier`: raw for a test binary built from
//   the package's source, since a policy test reads its package's source as text. A test build
//   takes the package's `tests/`, `benches/` and `examples/` raw whatever `ownTier` is;
// - every package they are compiled against (`closure`, from `cargo metadata`), each file at
//   `depTier` and without the package's `tests/`, `benches/` and `examples/`, which no dependant is
//   compiled from;
// - the files embedded at an include site outside a test module, anywhere in those packages, as
//   raw bytes. A file embedded from inside a test module is an input of the package's `lib` or
//   `bin` test binary alone, so only a `test` build that compiles the source under `cfg(test)`
//   takes it;
// - the files a package's `build.rs` reads from outside the package, from `BUILD_READS`.
//
// A dependency's tier is `shipped` for anything that reads the binary's text as well as running it,
// and `card` for a check that only runs programs, and for a test binary none of whose packages reads
// a reference card (`CARD_READERS`): `scan.ts` says what each tier leaves out.

import { type Graph, closure } from "./graph.ts";
import { partitionOf } from "./partition.ts";
import { type Tier, digest } from "./scan.ts";
import { type Tree, under } from "./tree.ts";

export interface Part {
  /** A repo-relative path, or a name in angle brackets for something that is not a file. */
  label: string;
  partition: string;
  tier: Tier;
  digest: string;
}

export interface Build {
  /** The packages compiled as this build's own. */
  own: string[];
  ownTier: Tier;
  /** The tier of everything the own packages are compiled against. */
  depTier: Tier;
  /** A test binary: the own packages' dev-dependencies, their `tests/`, `benches/` and `examples/`,
   * and the include sites in those, are its inputs. */
  test: boolean;
  /** Does a test build compile the own packages' source under `cfg(test)`, so an include site in a
   * test module there is an input? Omitted: yes. An integration test's build says no. */
  srcTest?: boolean;
}

/** What a package's `build.rs` reads from outside the package, by package. The files a `build.rs`
 * reads inside its own package are in the key already. */
export const BUILD_READS: Record<string, readonly string[]> = {
  "nvs-stdlib": ["docs/spec/02-php-migration.md", "tools/data/php-builtins.txt", "docs/reference/core"],
  "nvs-cli": ["LICENSE", "THIRD-PARTY-LICENSES.txt"],
};

/** The package the reference cards are declared in. */
export const CARD_HOME = "nvs-stdlib";

/** The packages whose code reads a reference card. A test binary built from none of them keys its
 * dependencies at `card`. `escape.ts`'s `cardReaders` fails `verify`'s `test` step on code anywhere
 * else that names a card type, or reads a `doc` field in a file that uses the registry. */
export const CARD_READERS: readonly string[] = ["nvs-cli", "nvs-lsp"];

/** The workspace files every build reads, whatever it compiles. */
const ROOT_MANIFESTS = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo/config.toml"];

/** A package's directories that no dependant is compiled from. */
const OWN_ONLY = ["tests", "benches", "examples"];

export class UnknownPackage extends Error {}

/** The parts of what `build` is compiled from, sorted by label. Throws `UnknownPackage` for a
 * package the graph does not name, and the caller keys the check on everything. */
export function builtFrom(tree: Tree, graph: Graph, build: Build): Part[] {
  const parts = new Map<string, Part>();
  const add = (label: string, tier: Tier, value?: string) => {
    if (parts.has(label)) return;
    parts.set(label, { label, partition: value === undefined ? partitionOf(label) : "toolchain", tier, digest: value ?? tree.digest(label, tier) });
  };
  add("<rustc>", "raw", digest(tree.toolchain));
  for (const rel of ROOT_MANIFESTS) if (tree.has(rel)) add(rel, "raw");

  const own = new Set(build.own);
  const deps = new Set<string>();
  for (const name of own) {
    if (!graph.has(name)) throw new UnknownPackage(`no workspace package is named ${name}`);
    for (const dep of closure(graph, name, build.test)) if (!own.has(dep)) deps.add(dep);
  }
  for (const pkg of graph.values()) add(`${pkg.dir}/Cargo.toml`, "raw");

  const shippedFiles = (name: string) => {
    const dir = graph.get(name)!.dir;
    return tree.under(dir).filter((rel) => !OWN_ONLY.some((d) => under(rel, `${dir}/${d}`)));
  };
  // Embedded files first, so a `.rs` file some source embeds is raw whatever tier its package is at.
  for (const name of [...own, ...deps]) {
    const dir = graph.get(name)!.dir;
    const ownTest = own.has(name) && build.test;
    const files = ownTest ? tree.under(dir) : shippedFiles(name);
    for (const rel of files) {
      const cfgTest = ownTest && (build.srcTest !== false || OWN_ONLY.some((d) => under(rel, `${dir}/${d}`)));
      for (const site of tree.includes(rel)) {
        if (site.inTest && !cfgTest) continue;
        const embedded = tree.under(site.path);
        for (const f of embedded) add(f, "raw");
        // A missing file is an input too, so its arrival moves the key.
        if (embedded.length === 0) add(site.path, "raw");
      }
    }
    for (const top of BUILD_READS[name] ?? []) for (const f of tree.under(top)) add(f, "raw");
  }
  for (const name of own) {
    const dir = graph.get(name)!.dir;
    if (!build.test) for (const rel of shippedFiles(name)) add(rel, build.ownTier);
    else for (const rel of tree.under(dir)) add(rel, OWN_ONLY.some((d) => under(rel, `${dir}/${d}`)) ? "raw" : build.ownTier);
  }
  // A test binary with no card reader among its packages cannot print a card, so an edit to one
  // does not move its key.
  const readsCards = [...own, ...deps].some((name) => CARD_READERS.includes(name)) || own.has(CARD_HOME);
  const depTier = build.test && build.depTier === "shipped" && !readsCards ? "card" : build.depTier;
  for (const name of [...deps].sort()) for (const rel of shippedFiles(name)) add(rel, depTier);
  return [...parts.values()].sort((a, b) => (a.label < b.label ? -1 : a.label > b.label ? 1 : 0));
}

/** The key of `parts` under `name`: two checks with the same inputs still key apart. */
export function keyOf(name: string, parts: Part[]): string {
  return digest(name, ...parts.map((p) => `${p.label}\x01${p.digest}`));
}

/** The build a package's test binary is compiled from. The `lib` and `bin` test binaries compile the
 * package's own source under `cfg(test)`, so it is raw. An integration test compiles its own file
 * under `tests/` and links the library built without `cfg(test)`, so the library is `shipped`; what
 * it reads of the source as text is an include site or a recorded read. */
export function testBuild(pkg: string, kind = "lib"): Build {
  if (kind === "lib" || kind === "bin") return { own: [pkg], ownTier: "raw", depTier: "shipped", test: true };
  return { own: [pkg], ownTier: "shipped", depTier: "shipped", test: true, srcTest: false };
}
