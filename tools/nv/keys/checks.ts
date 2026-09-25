// Every unit a key answers for, and what each one reads: the test binaries `verify` runs, and each
// check in the live goal's plan, its record with the floor carried in (`lib/chain.ts`'s `goalPlan`). A unit returns `Part[]` over a tree, so `nv why` prints it and
// `nv impact --probe` compares it before and after an edit, and neither has a copy of the rule.
//
// A unit's `source` says where its verdict is kept and how narrow its key is:
//
// - `verify record`: a test check of any shape, and a `.nvst` suite over the conformance or
//   differential tree. `verify` keeps one verdict per test binary and per case tree, and the unit's
//   key is the union of theirs.
// - `binary key`: a test binary. What it is compiled from, `builtFrom` with `testBuild`, and the
//   paths it last recorded opening at run time. A binary `tools/data/impact-wide.txt` lists keys on
//   everything.
// - `package key`: a check that builds or runs a program. Its build is `builtFrom` at the tier the
//   check reads, plus the paths the program opens. A check that only runs programs builds at `card`,
//   since no program prints a card: the program kinds, the legs, the suites, fuzz, TSan and the
//   database matrix. An `nvs` command, the editor's host run and a cost margin build at `shipped`.
// - `observed`: a check over what it was last seen to read. A proofs group, the unit `proofs: <group>` for
//   each `--group` an `nv proofs --run` or `--verify` check names, keys on the proof binary, what the
//   roster and the audit read for every group, and the example, attack and bench paths `nv proofs`
//   last recorded for that group. A check that names groups is the union of its groups.
// - `partitions`: a check whose reads are whole directories or files it names.
// - `everything`: anything else, and every doubt.
//
// Fuzz builds the crates its target's source `use`s, over `fuzz/**`. TSan builds the workspace
// packages its script names, raw and with their test modules. The database matrix builds each
// `cargo test` argument list its script writes, and an integration test among them builds its
// package's library without its test modules. The matrix is `bun nv db-matrix`, and its lists are
// read from that command's module. Both key on what runs them, every `Cargo.toml` and `examples/`,
// and the matrix on `tests/db/` too. What runs the matrix is the whole `nv` program, since
// `tools/nv/main.ts` loads every command.
//
// A probe names units by `<role>: <name>`, and `roleOf` says what a check's role is.

import { existsSync, readFileSync } from "node:fs";
import { goalPlan, liveGoal } from "../lib/chain.ts";
import { abs } from "../lib/paths.ts";
import { type Graph, byCrate, testBinaries } from "./graph.ts";
import { type Build, type Part, UnknownPackage, builtFrom, testBuild } from "./key.ts";
import { OTHER, PARTITIONS, STATE, partitionOf } from "./partition.ts";
import { TIERS, digest } from "./scan.ts";
import type { Tree } from "./tree.ts";

export type Source = "verify record" | "binary key" | "observed" | "package key" | "partitions" | "everything";

/** What a unit is: a test binary, a leg, or the kind of thing a check runs. A probe names units by
 * `<role>: <name>`. */
export type Role = "binary" | "leg" | "program" | "suite" | "test" | "nvs" | "editor" | "vsix" | "fuzz" | "tsan" | "db-matrix" | "nv" | "grep" | "other";

/** One check, as a goal record writes it. */
export type Check = { kind: string } & Record<string, unknown>;

export interface Unit {
  /** A check's name, or a test binary's `<package> <kind> <target>`. */
  name: string;
  source: Source;
  role: Role;
  /** The `[[check]]` this unit is; none for a test binary or a leg. */
  check?: Check;
  parts(tree: Tree): Part[];
}

/** Everything a unit's key is read from besides the tree. */
export interface Records {
  graph: Graph;
  checks: Check[];
  /** Each test binary's run-time reads, repo-relative, as `verify` last recorded them. */
  reads: Map<string, string[]>;
  /** The test binaries keyed on everything. */
  wide: Set<string>;
  /** Each proofs group's own paths, as `nv proofs` last recorded them. */
  proofReads: Map<string, string[]>;
}

const READS = ".agent-tmp/impact-reads.json";
const WIDE = "tools/data/impact-wide.txt";
/** Each proofs group's example and attack directories and bench file, which `nv proofs` writes. */
export const PROOF_READS = ".loop/proof-reads.json";

/** The two memos that are a whole leg rather than a check, keyed like a program. */
export const LEGS = ["wsl leg", "valgrind sweep"];
const PROGRAM_KINDS = new Set(["exact", "ordered", "contains", "min-bytes"]);
const TEST_TREES = ["tests", "conformance", "differential", "hostile", "lsp-cases"];
/** What a program opens besides its build. */
const PROGRAM_READS = ["docs", "examples", "tests"];
/** What an `nvs` command opens besides its build. */
const NVS_COMMAND_READS = ["docs", "examples", ...TEST_TREES];
/** The case trees whose verdicts `verify` keeps. */
const VERIFY_TREES = ["tests/conformance", "tests/differential"];
/** Every partition, for a unit keyed on everything. */
export const EVERYTHING = [...Object.keys(PARTITIONS), OTHER, STATE];

const FUZZ = "cargo +nightly fuzz run";
const TSAN = "tools/tsan.sh";
const DB_MATRIX = "tools/nv/cmd/db-matrix.ts";
/** What `bun nv <command>` runs besides the command's own module. */
const NV_PROGRAM = ["tools/nv", "package.json", "bun.lock", "tsconfig.json"];

function readJson(rel: string): unknown {
  try {
    return JSON.parse(readFileSync(abs(rel), "utf8"));
  } catch {
    return undefined;
  }
}

/** The live goal's plan's checks, in plan order; none when no goal is live or it has no record. */
function liveChecks(): Check[] {
  const live = liveGoal();
  return live === null ? [] : ((goalPlan(live.slug)?.checks ?? []) as Check[]);
}

/** The records on disk. A missing one reads as empty, which only ever widens a key. `live` false reads
 * no checks, for a caller that keys only the test binaries. */
export function loadRecords(graph: Graph, live = true): Records {
  const doc = { check: live ? liveChecks() : [] };
  const reads = new Map<string, string[]>();
  const got = readJson(READS);
  if (got && typeof got === "object") {
    for (const [name, v] of Object.entries(got as Record<string, { reads?: unknown }>)) {
      if (Array.isArray(v?.reads)) reads.set(name, v.reads.filter((r): r is string => typeof r === "string"));
    }
  }
  const wide = new Set<string>();
  if (existsSync(abs(WIDE))) {
    for (const line of readFileSync(abs(WIDE), "utf8").split(/\r?\n/)) {
      if (!line.trim() || line.startsWith("#")) continue;
      wide.add(line.split("  --")[0]!.trim());
    }
  }
  const proofReads = new Map<string, string[]>();
  const recorded = readJson(PROOF_READS);
  if (recorded && typeof recorded === "object") {
    for (const [group, v] of Object.entries(recorded as Record<string, unknown>)) {
      if (Array.isArray(v)) proofReads.set(group, v.filter((p): p is string => typeof p === "string"));
    }
  }
  return { graph, checks: doc.check ?? [], reads, wide, proofReads };
}

/** A check's name, as the driver files its verdict: its `name`, else its `file`, else what it runs. */
export function checkName(c: Check): string {
  if (typeof c.name === "string") return c.name;
  if (typeof c.file === "string") return c.file;
  return [c.kind, ...strings(c.argv ?? c.args)].join(" ");
}

function strings(v: unknown): string[] {
  return Array.isArray(v) ? v.filter((a): a is string => typeof a === "string") : [];
}

// ---- parts, memoized per tree ----------------------------------------------------------------------

const memo = new WeakMap<Tree, Map<string, Part[]>>();

function cached(tree: Tree, key: string, make: () => Part[]): Part[] {
  let m = memo.get(tree);
  if (!m) memo.set(tree, (m = new Map()));
  let got = m.get(key);
  if (!got) m.set(key, (got = make()));
  return got;
}

const byPartition = new WeakMap<Tree, Map<string, string[]>>();

function partitionFiles(tree: Tree, name: string): string[] {
  let m = byPartition.get(tree);
  if (!m) {
    m = new Map();
    for (const rel of tree.files) {
      const p = partitionOf(rel);
      let list = m.get(p);
      if (!list) m.set(p, (list = []));
      list.push(rel);
    }
    byPartition.set(tree, m);
  }
  return m.get(name) ?? [];
}

/** One part for a whole partition: the bytes of every file in it. */
function partition(tree: Tree, name: string): Part[] {
  return cached(tree, `partition\0${name}`, () => {
    const files = partitionFiles(tree, name);
    return [{ label: `<${name}>`, partition: name, tier: "raw", digest: digest(name, ...files.map((f) => `${f}\x01${tree.raw(f)}`)) }];
  });
}

/** One part for a file, or for every file under a directory; a path with none is `absent`. The
 * root, `.`, is every file in the tree. */
function path(tree: Tree, top: string): Part[] {
  if (top === ".") top = "";
  return cached(tree, `path\0${top}`, () => {
    if (top === "") return [{ label: "./", partition: OTHER, tier: "raw", digest: digest("", ...tree.files.map((f) => `${f}\x01${tree.raw(f)}`)) }];
    const files = tree.under(top);
    const label = files.length === 1 && files[0] === top ? top : `${top}/`;
    return [{ label, partition: partitionOf(top + (label.endsWith("/") ? "/" : "")), tier: "raw", digest: digest(top, ...(files.length ? files.map((f) => `${f}\x01${tree.raw(f)}`) : ["absent"])) }];
  });
}

/** Every `Cargo.toml` in the tree, workspace member or not. */
function manifests(tree: Tree): Part[] {
  return cached(tree, "manifests", () => {
    const files = tree.files.filter((f) => f === "Cargo.toml" || f.endsWith("/Cargo.toml"));
    return [{ label: "<every Cargo.toml>", partition: "crates", tier: "raw", digest: digest("manifests", ...files.map((f) => `${f}\x01${tree.raw(f)}`)) }];
  });
}

/** Is this a key on everything? Only `everything` puts the `state` partition in a key. */
export function isWide(parts: Part[]): boolean {
  return parts.some((p) => p.label === `<${STATE}>`);
}

function everything(tree: Tree): Part[] {
  return cached(tree, "everything", () => EVERYTHING.flatMap((p) => partition(tree, p)));
}

function build(tree: Tree, graph: Graph, b: Build): Part[] {
  return cached(tree, `build\0${JSON.stringify(b)}`, () => {
    try {
      return builtFrom(tree, graph, b);
    } catch (e) {
      if (e instanceof UnknownPackage) return everything(tree);
      throw e;
    }
  });
}

/** The parts in `lists`, sorted by label. Of two parts with one label, the one at the wider tier is
 * kept, and of two at one tier the first. */
function union(...lists: Part[][]): Part[] {
  const out = new Map<string, Part>();
  for (const list of lists) {
    for (const p of list) {
      const had = out.get(p.label);
      if (!had || TIERS.indexOf(p.tier) < TIERS.indexOf(had.tier)) out.set(p.label, p);
    }
  }
  return [...out.values()].sort((a, b) => (a.label < b.label ? -1 : a.label > b.label ? 1 : 0));
}

/** What `nvs` is built from, at the tier its caller reads. */
function program(tier: "card" | "shipped"): Build {
  return { own: ["nvs-cli"], ownTier: tier, depTier: tier, test: false };
}

// ---- the units -------------------------------------------------------------------------------------

/** Every unit: the test binaries, the legs, then each check in the live plan's order. */
export function units(r: Records): Unit[] {
  const out: Unit[] = [];
  const binaries = new Map<string, string>();
  for (const pkg of [...r.graph.keys()].sort()) {
    for (const b of testBinaries(r.graph, pkg)) {
      binaries.set(b.name, pkg);
      out.push({ name: b.name, source: r.wide.has(b.name) ? "everything" : "binary key", role: "binary", parts: (t) => binaryParts(t, r, pkg, b.name) });
    }
  }
  for (const leg of LEGS) out.push({ name: leg, source: "package key", role: "leg", parts: (t) => programParts(t, r.graph, PROGRAM_READS) });
  for (const c of r.checks) out.push(checkUnit(r, c));
  const groups = new Set(r.checks.flatMap((c) => proofGroups(c) ?? []));
  for (const group of groups) out.push({ name: `proofs: ${group}`, source: "observed", role: "nv", parts: (t) => proofParts(t, r, group) });
  return out;
}

/** The groups an `nv proofs --run` or `--verify` check runs, each a unit of its own. `undefined` when it
 * runs no group as a whole: it names none, or `--id` narrows it to one feature. */
export function proofGroups(c: Check): string[] | undefined {
  const argv = strings(c.argv);
  if (c.kind !== "command" || argv[0] !== "bun" || argv[1] !== "nv" || argv[2] !== "proofs") return undefined;
  if (!argv.includes("--run") && !argv.includes("--verify")) return undefined;
  if (argv.some((a) => a === "--id" || a.startsWith("--id="))) return undefined;
  const groups: string[] = [];
  argv.forEach((a, i) => {
    if (a === "--group" && argv[i + 1] !== undefined) groups.push(argv[i + 1]!);
    else if (a.startsWith("--group=")) groups.push(a.slice("--group=".length));
  });
  return groups.length > 0 ? groups : undefined;
}

/** What `nv proofs` reads for every group: its own code and runtime, the policy, the help backlog and
 * the perf ledger. */
const PROOF_INPUTS = ["tools/nv", "package.json", "bun.lock", "data/proofs", "docs/perf/members.ndjson"];
/** The partitions it reads for every group: the roster's chapters and spec, the registry and its tables,
 * and each `covers:` marker and plain call in the case trees and in `crates/`. */
const PROOF_PARTITIONS = ["crates", "crate-tests", "docs", "conformance", "differential"];

/** A proofs group's parts: the proof binary, what every group reads, and the group's own paths. A group
 * with no record keys on everything until it has run once. */
function proofParts(tree: Tree, r: Records, group: string): Part[] {
  const own = r.proofReads.get(group);
  if (!own) return everything(tree);
  return union(
    build(tree, r.graph, program("shipped")),
    ...PROOF_PARTITIONS.map((p) => partition(tree, p)),
    ...[...PROOF_INPUTS, ...own].map((p) => path(tree, p)),
  );
}

/** A test binary's parts. Its name is `<package> <kind> <target>`, as `testBinaries` writes it. */
function binaryParts(tree: Tree, r: Records, pkg: string, name: string): Part[] {
  const built = build(tree, r.graph, testBuild(pkg, name.split(" ")[1]));
  if (r.wide.has(name)) return union(built, everything(tree));
  return union(built, ...(r.reads.get(name) ?? []).map((rel) => path(tree, rel)));
}

function programParts(tree: Tree, graph: Graph, reads: string[], tier: "card" | "shipped" = "card"): Part[] {
  return union(build(tree, graph, program(tier)), ...reads.map((p) => partition(tree, p)));
}

/** The test binaries a `cargo test -p <pkg>` check runs: those its `--lib`, `--bin` or `--test`
 * names, or all of the package's. */
export function testTargets(graph: Graph, args: string[]): { pkg: string; names: string[] } | undefined {
  const at = args.indexOf("-p");
  if (args[0] !== "test" || at < 0 || !args[at + 1]) return undefined;
  const pkg = args[at + 1]!;
  let all = testBinaries(graph, pkg);
  const flag = (f: string) => (args.includes(f) ? args[args.indexOf(f) + 1] : undefined);
  const bin = flag("--bin");
  const test = flag("--test");
  if (args.includes("--lib")) all = all.filter((b) => b.target.kind === "lib");
  else if (bin !== undefined) all = all.filter((b) => b.target.kind === "bin" && b.target.name === bin);
  else if (test !== undefined) all = all.filter((b) => b.target.kind === "test" && b.target.name === test);
  return { pkg, names: all.map((b) => b.name) };
}

/** Does this command run what `needle` names? A `git grep` that only quotes it does not. */
function runs(argv: string[], needle: string): boolean {
  return argv.length > 0 && argv[0] !== "git" && argv.some((a) => a.includes(needle));
}

/** Whether this command runs the database matrix, `bun nv db-matrix`. */
function runsDbMatrix(argv: string[]): boolean {
  return argv[0] === "bun" && argv[1] === "nv" && argv[2] === "db-matrix";
}

/** The workspace packages a file names as a whole word: what a script builds. */
export function namedIn(tree: Tree, graph: Graph, rel: string): string[] {
  if (!tree.has(rel)) return [];
  const text = tree.text(rel);
  return [...graph.keys()].filter((n) => new RegExp(`(?<![\\w-])${n.replace(/-/g, "\\-")}(?![\\w-])`).test(text)).sort();
}

/** The builds of the `cargo test` argument lists a script writes as list literals, `["-p", "nvs-db"]`,
 * each at `card`. One that names `--test` builds that integration test over the package's library;
 * any other builds the package's source under `cfg(test)`. Empty when a list names no package the
 * graph has, and the caller keys on everything. */
export function suitesIn(tree: Tree, graph: Graph, rel: string): Build[] {
  if (!tree.has(rel)) return [];
  const out: Build[] = [];
  for (const m of tree.text(rel).matchAll(/\[\s*("[^"\n]*"(?:\s*,\s*"[^"\n]*")*)\s*,?\s*\]/g)) {
    const args = [...m[1]!.matchAll(/"([^"]*)"/g)].map((a) => a[1]!);
    const flag = (f: string) => (args.includes(f) ? args[args.indexOf(f) + 1] : undefined);
    const bin = flag("--bin");
    if (flag("-p") === undefined && bin === undefined) continue;
    const pkg = flag("-p") ?? [...graph.values()].find((p) => p.targets.some((t) => t.kind === "bin" && t.name === bin))?.name;
    if (pkg === undefined || !graph.has(pkg)) return [];
    const integration = flag("--test") !== undefined;
    out.push({ own: [pkg], ownTier: integration ? "card" : "raw", depTier: "card", test: true, ...(integration ? { srcTest: false } : {}) });
  }
  return out;
}

/** The workspace packages a Rust source `use`s or names by path. */
export function usedBy(tree: Tree, graph: Graph, rel: string): string[] {
  const out = new Set<string>();
  for (const m of tree.text(rel).matchAll(/(?<![\w:])(nvs_\w+)(?=::|\s*[;{])/g)) {
    const pkg = byCrate(graph, m[1]!);
    if (pkg) out.add(pkg);
  }
  return [...out].sort();
}

/** What a check is, for a probe to name a class of units by. */
export function roleOf(c: Check): Role {
  if (PROGRAM_KINDS.has(c.kind)) return "program";
  if (c.kind === "nvs-suite") return "suite";
  if (c.kind === "cargo-named") return "test";
  if (c.kind !== "command") return "other";
  const argv = strings(c.argv);
  const cwd = typeof c.cwd === "string" ? c.cwd : ".";
  if (argv[0] === "{nvs}") return "nvs";
  if (argv[0] === "npm" && cwd.startsWith("editors/")) return argv.includes("package") ? "vsix" : "editor";
  if (runs(argv, FUZZ)) return "fuzz";
  if (runs(argv, TSAN)) return "tsan";
  if (runsDbMatrix(argv)) return "db-matrix";
  if (argv[0] === "bun") return "nv";
  if (argv[0] === "git" && argv[1] === "grep") return "grep";
  return "other";
}

function checkUnit(r: Records, c: Check): Unit {
  const name = checkName(c);
  const role = roleOf(c);
  const unit = (source: Source, parts: (t: Tree) => Part[]): Unit => ({ name, source, role, check: c, parts });
  const wide = unit("everything", everything);
  const g = r.graph;

  if (PROGRAM_KINDS.has(c.kind)) return unit("package key", (t) => programParts(t, g, PROGRAM_READS));

  if (c.kind === "nvs-suite") {
    const paths = strings(c.args).slice(1).filter((a) => !a.startsWith("-")).map((a) => a.replace(/\/+$/, ""));
    if (paths.length > 0 && paths.every((p) => VERIFY_TREES.some((v) => p === v || p.startsWith(v + "/")))) {
      return unit("verify record", (t) => union(build(t, g, program("card")), ...paths.map((p) => path(t, p))));
    }
    const trees = new Set<string>();
    for (const p of paths) {
      const part = partitionOf(p + "/");
      for (const tr of part === "tests" ? TEST_TREES : [part]) trees.add(tr);
    }
    if (paths.length === 0) for (const tr of TEST_TREES) trees.add(tr);
    return unit("package key", (t) => programParts(t, g, [...new Set([...PROGRAM_READS, ...trees])]));
  }

  if (c.kind === "cargo-named") {
    const targets = testTargets(g, strings(c.args));
    if (!targets || targets.names.length === 0) return wide;
    return unit("verify record", (t) => union(...targets.names.map((n) => binaryParts(t, r, targets.pkg, n))));
  }

  if (c.kind !== "command") return wide;
  const argv = strings(c.argv);
  const cwd = typeof c.cwd === "string" ? c.cwd : ".";

  if (argv[0] === "{nvs}") return unit("package key", (t) => programParts(t, g, NVS_COMMAND_READS, "shipped"));

  if (argv[0] === "npm" && cwd.startsWith("editors/")) {
    // The host run starts `nvs lsp`; packaging and the headless suite read the extension alone.
    if (argv.includes("{nvs}")) return unit("package key", (t) => union(build(t, g, program("shipped")), path(t, "editors")));
    return unit("partitions", (t) => path(t, "editors"));
  }

  if (runs(argv, FUZZ)) {
    const target = argv.join(" ").match(/fuzz run (\S+)/)?.[1];
    const src = `fuzz/fuzz_targets/${target}.rs`;
    return unit("package key", (t) => {
      const own = target && t.has(src) ? usedBy(t, g, src) : [];
      if (own.length === 0) return everything(t);
      return union(build(t, g, { own, ownTier: "card", depTier: "card", test: false }), path(t, "fuzz"));
    });
  }

  // What these binaries open at run time was read off their source rather than taken from their
  // recorded reads: `manifest_policy` walks the root, and what it opens there is every
  // `Cargo.toml`, so the manifests stand in for the root its record names.
  if (runs(argv, TSAN)) {
    return unit("package key", (t) => {
      const own = namedIn(t, g, TSAN);
      if (own.length === 0) return everything(t);
      return union(build(t, g, { own, ownTier: "raw", depTier: "card", test: true }), path(t, TSAN), manifests(t), path(t, "examples"));
    });
  }
  if (runsDbMatrix(argv)) {
    return unit("package key", (t) => {
      const builds = suitesIn(t, g, DB_MATRIX);
      if (builds.length === 0) return everything(t);
      return union(...builds.map((b) => build(t, g, b)), ...NV_PROGRAM.map((p) => path(t, p)), manifests(t), path(t, "examples"), path(t, "tests/db"));
    });
  }

  const groups = proofGroups(c);
  if (groups) return unit("observed", (t) => union(...groups.map((gr) => proofParts(t, r, gr))));

  // A `git grep` over literal paths reads those paths and nothing else.
  if (argv[0] === "git" && argv[1] === "grep" && argv.includes("--")) {
    const specs = argv.slice(argv.indexOf("--") + 1);
    if (specs.length > 0 && specs.every((s) => !/[*?[:]/.test(s))) {
      return unit("partitions", (t) => union(...specs.map((s) => path(t, s.replace(/\/+$/, "")))));
    }
  }
  return wide;
}

