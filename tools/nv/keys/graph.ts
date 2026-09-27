// The workspace's packages and what each is compiled against, read from `cargo metadata` and never
// from a parse of a `Cargo.toml`: cargo is what resolves a manifest, so cargo is what says what it
// means.

import { run } from "../lib/proc.ts";
import { ROOT, rel } from "../lib/paths.ts";

export type DepKind = "normal" | "dev" | "build";

export interface Target {
  /** `lib`, `bin`, `test`, `bench`, `example` or `custom-build`: the first kind cargo gives. */
  kind: string;
  name: string;
  /** The target's root source, repo-relative. */
  src: string;
  /** Does `cargo test` build a test binary from it? */
  test: boolean;
}

export interface Package {
  name: string;
  /** The package's directory, repo-relative. */
  dir: string;
  deps: Map<string, DepKind>;
  targets: Target[];
}

export type Graph = Map<string, Package>;

interface RawTarget {
  kind: string[];
  name: string;
  src_path: string;
  test?: boolean;
}

interface RawPackage {
  name: string;
  manifest_path: string;
  dependencies: { name: string; kind: string | null; rename?: string | null }[];
  targets: RawTarget[];
}

/** The graph of the workspace whose manifest is `manifest` (the root one when omitted), or `null`
 * when cargo cannot say. */
export async function metadata(manifest?: string): Promise<Graph | null> {
  const argv = ["cargo", "metadata", "--format-version", "1", "--offline", "--no-deps"];
  if (manifest) argv.push("--manifest-path", manifest);
  let doc: { packages: RawPackage[] };
  try {
    const r = await run(argv, { timeoutMs: 120_000 });
    if (r.code !== 0) return null;
    doc = JSON.parse(r.stdout);
  } catch {
    return null;
  }
  const names = new Set(doc.packages.map((p) => p.name));
  const out: Graph = new Map();
  for (const p of doc.packages) {
    const dir = rel(p.manifest_path.replace(/[\\/]Cargo\.toml$/, ""), ROOT);
    if (dir.startsWith("..")) return null;
    const deps = new Map<string, DepKind>();
    for (const d of p.dependencies) {
      if (!names.has(d.name)) continue;
      const kind = (d.kind ?? "normal") as DepKind;
      // One package named twice keeps the kind that reaches furthest.
      if (deps.get(d.name) !== "normal") deps.set(d.name, kind);
    }
    const targets = p.targets.map((t) => ({
      kind: t.kind[0] === "proc-macro" || t.kind[0]?.endsWith("lib") ? "lib" : (t.kind[0] ?? ""),
      name: t.name,
      src: rel(t.src_path, ROOT),
      test: t.test !== false,
    }));
    out.set(p.name, { name: p.name, dir, deps, targets });
  }
  return out;
}

/** Every workspace package `pkg`'s test binaries are compiled against: its own dependencies of
 * every kind, then theirs without the dev ones, which cargo builds for the package under test alone.
 * With `dev` false, `pkg`'s own dev-dependencies are left out too: that is what its shipped build
 * is compiled against. */
export function closure(graph: Graph, pkg: string, dev = true): Set<string> {
  const seen = new Set<string>();
  const todo: [string, boolean][] = [[pkg, dev]];
  while (todo.length > 0) {
    const [name, own] = todo.pop()!;
    for (const [dep, kind] of graph.get(name)?.deps ?? []) {
      if ((kind !== "dev" || own) && !seen.has(dep) && dep !== pkg) {
        seen.add(dep);
        todo.push([dep, false]);
      }
    }
  }
  return seen;
}

/** The package a crate name (`nvs_syntax`) is, or `undefined`. */
export function byCrate(graph: Graph, crate: string): string | undefined {
  for (const name of graph.keys()) if (name.replace(/-/g, "_") === crate) return name;
  return undefined;
}

/** The test binaries `cargo test` builds for `pkg`, named as `verify` names them:
 * `<package> <kind> <target>`. */
export function testBinaries(graph: Graph, pkg: string): { name: string; target: Target }[] {
  const p = graph.get(pkg);
  if (!p) return [];
  return p.targets
    .filter((t) => t.test && (t.kind === "lib" || t.kind === "bin" || t.kind === "test"))
    .map((t) => ({ name: `${pkg} ${t.kind} ${t.kind === "lib" ? t.name.replace(/-/g, "_") : t.name}`, target: t }));
}

/** Every test binary of the workspace, named as `testBinaries` names them. */
export function allTestBinaries(graph: Graph): string[] {
  return [...graph.keys()].sort().flatMap((pkg) => testBinaries(graph, pkg).map((b) => b.name));
}

/** The cargo flag that names one target of each kind. */
export const TARGET_FLAGS: Record<string, string> = { lib: "--lib", bin: "--bin", test: "--test", example: "--example", bench: "--bench" };

/** The cargo target filters that build the test binaries `names` (`<package> <kind> <target>`) and as
 * few others as a filter can: `--lib` builds every package's library tests, since a filter cannot name
 * one package, and `--test <name>` builds that target in every package that has one. */
export function targetFilters(names: string[]): string[] {
  const flags = new Set<string>();
  for (const n of names) {
    const [, kind, target] = n.split(" ");
    if (kind === "lib") flags.add("--lib");
    else if (kind && target && TARGET_FLAGS[kind]) flags.add(`${TARGET_FLAGS[kind]} ${target}`);
  }
  return [...flags].sort().flatMap((f) => f.split(" "));
}
