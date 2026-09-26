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
//   everything but the files a session writes after `nv verify` (`WRAP_WRITES`).
// - `package key`: a check that builds or runs a program. Its build is `builtFrom` at the tier the
//   check reads, plus the paths the program opens. A check that only runs programs builds at `card`,
//   since no program prints a card: the program kinds, the legs, the suites, fuzz, TSan and the
//   database matrix. The editor's host run and a cost margin build at `shipped`, and so does an `nvs`
//   command that prints a card; any other `nvs` command builds at `card`, and one that opens only its
//   configuration and the paths it names keys on those (`nvsCommandParts`).
// - `observed`: a check over what it was last seen to read. A proofs group, the unit `proofs: <group>` for
//   each `--group` an `nv proofs --run` or `--verify` check names, keys on the proof binary, the
//   `nv proofs` modules, what the roster and the audit read for every group, the markers and calls it
//   scans in the case trees and the crates' tests (`proofs/markers.ts`), and the example, attack
//   and bench paths `nv proofs` last recorded for that group. A check that names groups is the union of
//   its groups. Any other `bun nv` check keys on the modules its command loads (`modules.ts`) and on
//   what its processes read, tested, listed and started when the sweep last ran it (`lib/reads.ts`);
//   it keys on everything until it has run once, and whenever it started a program `spawnParts`
//   cannot key.
// - `partitions`: a check whose reads are whole directories or files it names.
// - `everything`: anything else, and every doubt.
//
// Fuzz builds the crates its target's source `use`s, over `fuzz/**`. TSan builds the workspace
// packages its script names, raw and with their test modules. The database matrix builds each
// `cargo test` argument list its script writes, and an integration test among them builds its
// package's library without its test modules. The matrix is `bun nv db-matrix`, and its lists are
// read from that command's module. Both key on what runs them, every `Cargo.toml` and `examples/`,
// and the matrix on `tests/db/` too. What runs the matrix is the `nv db-matrix` command's modules.
//
// A probe names units by `<role>: <name>`, and `roleOf` says what a check's role is.

import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { goalPlan, liveGoal } from "../lib/chain.ts";
import { abs } from "../lib/paths.ts";
import type { Reads } from "../lib/reads.ts";
import { CALL_ROOTS, MARKER_ROOTS, callsIn, markersIn } from "../proofs/markers.ts";
import { proofReadsSlot, SelectStore, testReads } from "../select/store.ts";
import { type Graph, byCrate, testBinaries } from "./graph.ts";
import { type Build, type Part, UnknownPackage, builtFrom, testBuild } from "./key.ts";
import { NV_MANIFESTS, commandModules } from "./modules.ts";
import { OTHER, PARTITIONS, STATE, partitionOf, wrapWritten } from "./partition.ts";
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
  /** For a test check keyed on its binaries, their names: its key is the union of theirs, so it moves
   * exactly when one of them does. */
  binaries?: string[];
  parts(tree: Tree): Part[];
}

/** Everything a unit's key is read from besides the tree. */
export interface Records {
  graph: Graph;
  checks: Check[];
  /** Each test binary's run-time reads, repo-relative: the paths its recorded footprint says it asked
   * `nvs_repo` for (`testReads`). */
  reads: Map<string, string[]>;
  /** The test binaries keyed on everything. */
  wide: Set<string>;
  /** Each proofs group's own paths, as `nv proofs` last recorded them in the selection store. */
  proofReads: Map<string, string[]>;
  /** What each `bun nv` check's processes read when it last ran, by `recordId`. */
  nvReads: Map<string, Reads>;
}

const WIDE = "tools/data/impact-wide.txt";
/** What each `bun nv` check read when it last ran, which the sweep writes (`lib/reads.ts`). */
export const NV_READS = ".loop/nv-reads.json";

/** The two memos that are a whole leg rather than a check, keyed like a program. */
export const LEGS = ["wsl leg", "valgrind sweep"];
const PROGRAM_KINDS = new Set(["exact", "ordered", "contains", "min-bytes"]);
const TEST_TREES = ["tests", "conformance", "differential", "hostile", "lsp-cases"];
/** What a program opens besides its build. */
const PROGRAM_READS = ["docs", "examples", "tests"];
/** What an `nvs` command opens besides its build. */
const NVS_COMMAND_READS = ["docs", "examples", ...TEST_TREES];
/** The `nvs` subcommands that print a reference card, and read the build at `shipped`. */
const CARD_COMMANDS = new Set(["meta", "doc", "agent", "lsp"]);
/** The `nvs` subcommands that open only their configuration and the paths their arguments name. `fmt`
 * and `check` walk their working directory when no path is named, so they are here only with one. */
const NAMED_READERS = new Set(["ast", "build", "config", "ctl", "doc", "meta", "queue", "schema", "service"]);
const WALKERS = new Set(["check", "fmt"]);
/** What every `nvs` command reads of its configuration: the file in its working directory, and the
 * databases' certificates and files that file names. */
const NVS_CONFIG_READS = ["nvs.toml", "nvs.local.toml", ".env", "tests/db"];

/** The card reader whose code reads a card only when its binary runs a `CARD_COMMANDS` subcommand. It
 * has no library, so an integration test of it reaches a card only by starting the binary with one. */
const CARD_BY_COMMAND = "nvs-cli";

/** Does this test source start a subcommand that prints a card: hold one as a whole string literal? A
 * source that declares a module file is taken to, since that file is not read here. */
function asksForCards(text: string): boolean {
  if (/^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*;/m.test(text)) return true;
  return [...text.matchAll(/"((?:[^"\\]|\\.)*)"/g)].some((m) => CARD_COMMANDS.has(m[1]!));
}

/** What an `{nvs} <argv>` check reads: its build, at `shipped` for a command that prints a card and at
 * `card` for any other, and what it opens. A command that opens only its configuration and named paths
 * keys on those, a named file standing for its directory since a program opens the files beside it. Any
 * other keys on every tree a command might walk. */
function nvsCommandParts(tree: Tree, graph: Graph, argv: string[], cwd: string): Part[] {
  const args = argv.slice(1);
  const sub = args.find((a) => !a.startsWith("-")) ?? "";
  const tier = CARD_COMMANDS.has(sub) ? "shipped" : "card";
  const trimmed = args.filter((a) => a !== sub && !a.startsWith("-")).map((a) => a.replace(/\\/g, "/").replace(/\/+$/, ""));
  const named = trimmed.filter((a) => tree.has(a) || tree.under(a).length > 0);
  const narrow = cwd === "." && (NAMED_READERS.has(sub) || (WALKERS.has(sub) && named.length > 0));
  if (!narrow) return programParts(tree, graph, NVS_COMMAND_READS, tier);
  // A file at the root stands for itself, since its directory is the whole tree.
  const opened = named.map((a) => (tree.has(a) && dirname(a) !== "." ? dirname(a).replace(/\\/g, "/") : a));
  return union(build(tree, graph, program(tier)), ...[...NVS_CONFIG_READS, ...opened].map((p) => path(tree, p)));
}
/** The case trees whose verdicts `verify` keeps. */
const VERIFY_TREES = ["tests/conformance", "tests/differential"];
/** Every partition a unit keyed on everything reads. `state` is not one: a wrap rewrites it every
 * session, nothing under `crates/` opens it, and a `bun nv` check that reads it keys on it through the
 * reads it was seen to make. */
export const EVERYTHING = [...Object.keys(PARTITIONS), OTHER];
/** The part that marks a key on everything. Its digest is constant, so it moves no key. */
const WIDE_MARK: Part = { label: "<everything>", partition: OTHER, tier: "raw", digest: "everything" };

const FUZZ = "cargo +nightly fuzz run";
const TSAN = "tools/tsan.sh";
const DB_MATRIX = "tools/nv/cmd/db-matrix.ts";

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
  // The test binaries' reads and the proof groups' paths live in the selection store.
  let reads = new Map<string, string[]>();
  const proofReads = new Map<string, string[]>();
  try {
    const store = new SelectStore();
    try {
      reads = testReads(store);
      for (const [slot, v] of store.verdictsWithPrefix(proofReadsSlot(""))) {
        const paths = JSON.parse(v.verdict) as unknown;
        if (Array.isArray(paths)) proofReads.set(slot.slice(proofReadsSlot("").length), paths.filter((p): p is string => typeof p === "string"));
      }
    } finally {
      store.close();
    }
  } catch {
    // A store that cannot be read reads as empty.
  }
  const wide = new Set<string>();
  if (existsSync(abs(WIDE))) {
    for (const line of readFileSync(abs(WIDE), "utf8").split(/\r?\n/)) {
      if (!line.trim() || line.startsWith("#")) continue;
      wide.add(line.split("  --")[0]!.trim());
    }
  }
  const nvReads = new Map<string, Reads>();
  const logged = readJson(NV_READS);
  if (logged && typeof logged === "object") {
    for (const [id, v] of Object.entries(logged as Record<string, Partial<Reads>>)) {
      if (v && Array.isArray(v.files) && Array.isArray(v.exists) && Array.isArray(v.dirs) && Array.isArray(v.spawns)) nvReads.set(id, v as Reads);
    }
  }
  return { graph, checks: doc.check ?? [], reads, wide, proofReads, nvReads };
}

/** Where a `bun nv` check's reads are kept: its directory and its argument list. */
export function recordId(cwd: string, argv: string[]): string {
  return [cwd, ...argv].join("\0");
}

/** Keeps what one run of a `bun nv` check read, in place of what it read before. */
export function saveNvReads(id: string, reads: Reads): void {
  const got = readJson(NV_READS);
  const all = got && typeof got === "object" && !Array.isArray(got) ? (got as Record<string, Reads>) : {};
  all[id] = reads;
  try {
    // Written beside and renamed over, so a reader never parses half a file.
    const path = abs(NV_READS);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(`${path}.${process.pid}`, `${JSON.stringify(all)}\n`);
    renameSync(`${path}.${process.pid}`, path);
  } catch {
    // A record that cannot be kept leaves the check keyed on everything, which is never unsafe.
  }
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
 * root, `.`, is every file in the tree, and `**\/<name>` every file called `<name>` anywhere in it
 * (`nvs_repo::named`). A directory leaves out the `state` files under it, which a wrap rewrites
 * every session: a reader of one names it. */
function path(tree: Tree, top: string): Part[] {
  if (top === ".") top = "";
  if (top.startsWith("**/")) {
    const name = top.slice(3);
    return cached(tree, `named\0${name}`, () => {
      const files = tree.files.filter((f) => f === name || f.endsWith(`/${name}`));
      return [{ label: top, partition: OTHER, tier: "raw", digest: digest(top, ...(files.length ? files.map((f) => `${f}\x01${tree.raw(f)}`) : ["absent"])) }];
    });
  }
  return cached(tree, `path\0${top}`, () => {
    const kept = (f: string) => f === top || partitionOf(f) !== STATE;
    if (top === "") return [{ label: "./", partition: OTHER, tier: "raw", digest: digest("", ...tree.files.filter(kept).map((f) => `${f}\x01${tree.raw(f)}`)) }];
    const files = tree.under(top).filter(kept);
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

/** Is this a key on everything? Only `everything` puts its mark in a key. */
export function isWide(parts: Part[]): boolean {
  return parts.some((p) => p.label === WIDE_MARK.label);
}

function everything(tree: Tree): Part[] {
  return cached(tree, "everything", () => [WIDE_MARK, ...EVERYTHING.flatMap((p) => partition(tree, p))]);
}

/** What a wide test binary keys on: everything but the files a session writes after `nv verify`
 * (`WRAP_WRITES`), which no wide binary's source names (`escape.ts`). This is what lets verify's run
 * of a wide binary still answer the sweep that follows the wrap and the perf recording. */
function everythingButWrap(tree: Tree): Part[] {
  return cached(tree, "everything-but-wrap", () => [
    WIDE_MARK,
    ...EVERYTHING.map((name): Part => {
      const files = partitionFiles(tree, name).filter((f) => !wrapWritten(f));
      return { label: `<${name}>`, partition: name, tier: "raw", digest: digest(name, ...files.map((f) => `${f}\x01${tree.raw(f)}`)) };
    }),
  ]);
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

/** What `nv proofs` reads for every group besides its own modules: the policy, the help backlog and
 * the perf ledger. */
const PROOF_INPUTS = ["data/proofs", "docs/perf/members.ndjson"];
/** The partitions it reads whole for every group: the roster's chapters and spec, and the registry and
 * its tables. */
const PROOF_PARTITIONS = ["crates", "docs"];

/** Each file's scan, by path and raw digest, so a second tree over an unchanged file reads it once. */
const scans = new Map<string, string>();

/** What `nv proofs` reads out of the case trees and the crates' Rust files, from the scanner it uses
 * (`proofs/markers.ts`): one part per tree, over each file's `covers:` markers and, in a case tree, its
 * calls. The rest of a file is not in it, so an edit that moves no marker and no call moves no key. */
function proofScan(tree: Tree): Part[] {
  return cached(tree, "proof-scan", () =>
    MARKER_ROOTS.map(([root, ext]) => {
      const calls = CALL_ROOTS.includes(root);
      const lines: string[] = [];
      for (const f of tree.under(root)) {
        if (!f.endsWith(ext) || f.split("/").includes("target")) continue;
        const at = `${f}\x01${tree.raw(f)}`;
        let got = scans.get(at);
        if (got === undefined) {
          const text = tree.text(f);
          got = [...markersIn(f, text).map(([name, label]) => `${name}\x02${label}`), ...(calls ? callsIn(text) : [])].join("\x03");
          scans.set(at, got);
        }
        if (got !== "") lines.push(`${f}\x01${got}`);
      }
      return { label: `<proof scan>${root}/`, partition: partitionOf(`${root}/`), tier: "raw" as const, digest: digest(root, ...lines) };
    }),
  );
}

/** A proofs group's parts: the proof binary, what every group reads, and the group's own paths. A group
 * with no record keys on everything until it has run once. */
function proofParts(tree: Tree, r: Records, group: string): Part[] {
  const own = r.proofReads.get(group);
  if (!own) return everything(tree);
  return union(
    build(tree, r.graph, program("shipped")),
    nvProgram(tree, "proofs"),
    ...PROOF_PARTITIONS.map((p) => partition(tree, p)),
    proofScan(tree),
    ...[...PROOF_INPUTS, ...own].map((p) => path(tree, p)),
  );
}

/** What `bun nv <name>` is loaded from: its modules and the manifests. Everything when no command has
 * that name. */
function nvProgram(tree: Tree, name: string): Part[] {
  return cached(tree, `nv\0${name}`, () => {
    const mods = commandModules(tree, name);
    if (mods === null) return everything(tree);
    return union(...[...mods, ...NV_MANIFESTS].map((p) => path(tree, p)));
  });
}

/** Whether `p` is a file, a directory or absent, and nothing about what it holds. */
function exists(tree: Tree, p: string): Part[] {
  const d = p === "." ? "dir" : tree.has(p) ? "file" : tree.under(p).length > 0 ? "dir" : "absent";
  return [{ label: `<exists>${p}`, partition: partitionOf(p), tier: "raw", digest: d }];
}

/** The name of every file under the directory `d`, and nothing about what they hold. */
function names(tree: Tree, d: string): Part[] {
  return cached(tree, `names\0${d}`, () => {
    const files = d === "." ? tree.files : tree.under(d);
    return [{ label: `<names>${d}/`, partition: d === "." ? OTHER : partitionOf(`${d}/`), tier: "raw", digest: digest(d, ...files) }];
  });
}

const BUILT_CLI = /^target\/[^/]+\/nvs(?:\.exe)?$/;

/** What a program a `bun nv` check started reads, or null when this cannot say, and the check keys on
 * everything. Another `bun nv` process records its own reads into the same log, so it adds nothing. */
export function spawnParts(tree: Tree, graph: Graph, argv: string[]): Part[] | null {
  const exe = (argv[0] ?? "").replace(/\\/g, "/");
  const name = exe.slice(exe.lastIndexOf("/") + 1).replace(/\.exe$/i, "").toLowerCase();
  const args = argv.slice(1);
  const sub = args.find((a) => !a.startsWith("-") && !a.startsWith("+"));
  const pair = (flag: string, value: string) => args.some((a, i) => a === flag && args[i + 1] === value);
  if (name === "bun") {
    if (args[0] === "nv" || (args[0] === "run" && args[1] === "nv")) return [];
    const script = (args.find((a) => !a.startsWith("-")) ?? "").replace(/\\/g, "/");
    return script.endsWith("tools/nv/main.ts") ? [] : null;
  }
  if (name === "git") {
    if (sub === "ls-files" || sub === "check-ignore") return names(tree, ".");
    if (sub === "config") return [];
    if (sub === "rev-parse" && args.every((a) => a === "rev-parse" || /^--(show-toplevel|show-prefix|git-dir|is-inside-work-tree)$/.test(a))) return [];
    if (sub === "grep" && args.includes("--")) {
      const specs = args.slice(args.indexOf("--") + 1);
      if (specs.length > 0 && specs.every((s) => !/[*?[:]/.test(s))) return union(...specs.map((s) => path(tree, s.replace(/\/+$/, ""))));
    }
    return null;
  }
  if (name === "rustc") return [{ label: "<rustc>", partition: "toolchain", tier: "raw", digest: digest(tree.toolchain) }];
  if (name === "cargo") {
    if (sub === "metadata") return manifests(tree);
    if (sub === "build" && (pair("-p", "nvs-cli") || pair("--bin", "nvs"))) return programParts(tree, graph, NVS_COMMAND_READS, "shipped");
    return null;
  }
  const rel = /^[A-Za-z]:\//.test(exe) || exe.startsWith("/") ? relPath(exe) : exe;
  return rel !== null && BUILT_CLI.test(rel) ? programParts(tree, graph, NVS_COMMAND_READS, "shipped") : null;
}

/** An absolute path as repo-relative, or null outside the repository. */
function relPath(p: string): string | null {
  const root = abs(".").replace(/\\/g, "/").replace(/\/$/, "");
  return p.toLowerCase().startsWith(`${root.toLowerCase()}/`) ? p.slice(root.length + 1) : null;
}

/** A `bun nv` check's parts over what it read when it last ran: its command's modules, each file it
 * read, each path it tested, each directory it listed, and what each program it started reads. A file
 * that is a directory in the tree was only ever `stat`ed, so it is keyed on its existence. */
function nvParts(tree: Tree, graph: Graph, name: string, rec: Reads): Part[] {
  const lists: Part[][] = [nvProgram(tree, name)];
  for (const f of rec.files) lists.push(!tree.has(f) && tree.under(f).length > 0 ? exists(tree, f) : path(tree, f));
  for (const f of rec.exists) lists.push(exists(tree, f));
  for (const d of rec.dirs) lists.push(names(tree, d));
  for (const argv of rec.spawns) {
    const got = spawnParts(tree, graph, argv);
    if (got === null) return everything(tree);
    lists.push(got);
  }
  return union(...lists);
}

/** A test binary's parts. Its name is `<package> <kind> <target>`, as `testBinaries` writes it. */
function binaryParts(tree: Tree, r: Records, pkg: string, name: string): Part[] {
  const kind = name.split(" ")[1];
  let b = testBuild(pkg, kind);
  if (kind === "test" && pkg === CARD_BY_COMMAND) {
    const src = testBinaries(r.graph, pkg).find((x) => x.name === name)?.target.src;
    if (src !== undefined && tree.has(src) && !asksForCards(tree.text(src))) b = { ...b, ownTier: "card", depTier: "card" };
  }
  const built = build(tree, r.graph, b);
  if (r.wide.has(name)) return union(built, everythingButWrap(tree));
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
    return { ...unit("verify record", (t) => union(...targets.names.map((n) => binaryParts(t, r, targets.pkg, n)))), binaries: targets.names };
  }

  if (c.kind !== "command") return wide;
  const argv = strings(c.argv);
  const cwd = typeof c.cwd === "string" ? c.cwd : ".";

  if (argv[0] === "{nvs}") return unit("package key", (t) => nvsCommandParts(t, g, argv, cwd));

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
      return union(...builds.map((b) => build(t, g, b)), nvProgram(t, "db-matrix"), manifests(t), path(t, "examples"), path(t, "tests/db"));
    });
  }

  const groups = proofGroups(c);
  if (groups) return unit("observed", (t) => union(...groups.map((gr) => proofParts(t, r, gr))));

  // Any other `bun nv` check keys on what it read when it last ran, and on everything until it has.
  if (argv[0] === "bun" && argv[1] === "nv") {
    const rec = r.nvReads.get(recordId(cwd, argv));
    if (rec !== undefined) return unit("observed", (t) => nvParts(t, g, argv[2] ?? "", rec));
    return wide;
  }

  // A `git grep` over literal paths reads those paths and nothing else.
  if (argv[0] === "git" && argv[1] === "grep" && argv.includes("--")) {
    const specs = argv.slice(argv.indexOf("--") + 1);
    if (specs.length > 0 && specs.every((s) => !/[*?[:]/.test(s))) {
      return unit("partitions", (t) => union(...specs.map((s) => path(t, s.replace(/\/+$/, "")))));
    }
  }
  return wide;
}

