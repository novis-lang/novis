// The live plan's checks as atoms. A plan `[[check]]` is a named group of atoms and its pass
// condition: the check is reached when the selection picks one of its atoms, and a check none of whose
// atoms is picked is not started. What each shape of check is made of:
//
// | check | its atoms | run as |
// |---|---|---|
// | a fixture (`exact`, `ordered`, `contains`, `min-bytes`) | `check:<id>` | `nvs run`, recorded |
// | `nvs-suite`, `nvs test <tree>` | the cases under the tree it names (`case:`) | the picked cases, through `nvs test --cases` |
// | `cargo-named`, `cargo test -p <crate>` narrowed to a target, or `--workspace` | the test binaries it names (`test:`) | each picked binary, whole |
// | `bun nv proofs --verify` or `--run` over groups | `nv:<id>` and the proof programs of its groups (`proof:`) | the command, which runs only its picked programs |
// | `bun nv selftest` | `step:nv`, `tsc` over the tools, and every tools test file (`nvtest:`) | `tsc` and each picked test file |
// | `bun test <file>` of the tools | `nvtest:<file>` | that file |
// | any other `bun nv` command | `nv:<id>` | the command, its reads recorded |
// | any other command | `check:<id>` | the command, recorded |
// | a heavy check (`driver/accept.ts` `isHeavy`) | `heavy:<id>` | the command, with the floor gate open |
//
// The atoms a test binary, a case, a test file or a proof program is are shared with `nv verify` and
// `nv proofs`, so a run of any one of them answers all three. A check's own atom (`check:`, `nv:`,
// `heavy:`) is defined by the check as the plan writes it, less its name and stage, so a changed `want`
// runs it again.
//
// **The heavy set builds what no recorded run builds**: the release profile, the fuzz and TSan builds
// inside WSL, the database matrix. What each run reads is observed where it can be: the footprint log
// of every `nvs` and test binary it starts, and the reads of a `bun nv` command. What it compiles is
// not, so a heavy check also holds `heavyKeys`: every Rust file of the crates it builds, as `fn:<file>#*`,
// which any item change in the file moves, and every directory of those crates, which a new file
// moves. That is the one prediction left in the selection, and it is named as one: `heavyCrates` reads
// the crates off the check's own arguments, the fuzz workspace's manifest, and the TSan and database
// matrix scripts. The two Linux legs run `nvs` inside WSL, which records nothing, so they are keyed the
// same way on what `nvs-cli` builds and the trees the fixtures read.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { type Check, isHeavy, PROGRAM_KINDS, proofGroups, subsetRun } from "../driver/accept.ts";
import { allTestBinaries, closure, type Graph, testBinaries } from "../keys/graph.ts";
import { digest } from "../keys/scan.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { caseFiles, caseId, nvTestFiles, nvTestId, proofFiles, proofId } from "./atoms.ts";
import { fileWild, spawnKeys, WILD } from "./keys.ts";
import { type Keyed, proofReadsSlot, type SelectStore } from "./store.ts";

/** How the sweep runs a check. */
export type How = "fixture" | "cases" | "tests" | "proofs" | "selftest" | "nvtest" | "nv" | "command" | "heavy";

/** The two Linux legs, each an atom of the heavy set. */
export const LEGS = ["wsl leg", "valgrind sweep"] as const;
export const legId = (leg: string) => `heavy:${leg}`;

/** A check as a group of atoms. */
export interface Grouped {
  how: How;
  /** Every atom the check is made of. */
  atoms: string[];
  /** The check's own atom and its definition, for a check that has one. */
  own?: { id: string; def: string };
  /** `cases`: the case files under the suite's trees. */
  cases?: string[];
  /** `tests`: the binaries, as `<package> <kind> <target>`. */
  tests?: string[];
  /** `proofs`: its groups. */
  groups?: string[];
  /** `proofs`: a group `nv proofs` has never recorded, or a new program no group's directory holds, so
   * which programs it runs is not known yet. */
  unknown?: boolean;
  /** `nvtest`: the file. */
  file?: string;
}

/** What `grouped` reads besides the check. */
export interface PlanContext {
  graph: Graph | null;
  /** Every `.nvst` case under `tests/`. */
  cases: string[];
  /** The tools' test files. */
  nvTests: string[];
  /** Each proofs group's example and attack directories, as `nv proofs` last recorded them. */
  groupDirs: Map<string, string[]>;
  /** Every proof program on disk (`atoms.ts` `proofFiles`). */
  proofs: string[];
  /** Set when a program on disk that the store never recorded is in no directory a group recorded: which
   * group it belongs to is not known until `nv proofs` runs, so every proofs group is taken as unknown. */
  homeless?: boolean;
}

/**
 * The context `grouped` reads, from the tree as it is and the store: every case, tools test and proof
 * program on disk, and each group's recorded directories. A new program in a directory no group recorded
 * sets `homeless`, so every `nv proofs` group check is reached until a run records it. That costs one
 * `nv proofs` process per batch of groups, each running only its picked programs, in every sweep until
 * then.
 */
export function planContext(store: SelectStore, graph: Graph | null, root: string = ROOT): PlanContext {
  const groupDirs = groupDirsOf(store);
  const proofs = proofFiles(root);
  const recorded = new Set(store.atoms("proof").map((a) => a.id.slice(6)));
  const dirs = [...groupDirs.values()].flat();
  const homeless = proofs.some((p) => !recorded.has(p) && !dirs.some((d) => p.startsWith(`${d}/`)));
  return { graph, cases: caseFiles(root), nvTests: nvTestFiles(root), groupDirs, proofs, homeless };
}

/** The groups' directories from the store: what each `nv proofs` run over a whole group recorded. */
export function groupDirsOf(store: SelectStore): Map<string, string[]> {
  const out = new Map<string, string[]>();
  for (const [slot, v] of store.verdictsWithPrefix(proofReadsSlot(""))) {
    try {
      const paths = JSON.parse(v.verdict) as unknown;
      if (Array.isArray(paths)) out.set(slot.slice(proofReadsSlot("").length), paths.filter((p): p is string => typeof p === "string" && !p.endsWith(".nvs")));
    } catch {
      // A slot that cannot be read names no directory, and the group is taken as never recorded.
    }
  }
  return out;
}

/** A check's own atom's definition: the check as the plan writes it, less what only names it. */
export function checkDef(c: Check): string {
  const { name: _name, stage: _stage, ...rest } = c;
  return digest(JSON.stringify(rest, Object.keys(rest).sort()));
}

/** The test binaries a `cargo test -p <pkg>` check runs: those its `--lib`, `--bin` or `--test` names,
 * or all of the package's. */
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

const NVS_TEST = /^tests\/[^/]+\/?$/;

/** The case trees an `nvs test` suite names, or null for a suite of another shape. */
function suiteTrees(args: string[]): string[] | null {
  if (args[0] !== "test") return null;
  const rest = args.slice(1);
  if (rest.length === 0 || rest.some((a) => a.startsWith("-") || !NVS_TEST.test(a.replace(/\\/g, "/")))) return null;
  return rest.map((a) => a.replace(/\\/g, "/").replace(/\/+$/, ""));
}

/** The check `c` as a group of atoms. */
export function grouped(c: Check, ctx: PlanContext): Grouped {
  const own = (kind: "check" | "nv" | "heavy") => ({ id: `${kind}:${c.id}`, def: checkDef(c) });
  const single = (how: How, kind: "check" | "nv" | "heavy"): Grouped => {
    const o = own(kind);
    return { how, atoms: [o.id], own: o };
  };
  if (isHeavy(c)) return single("heavy", "heavy");
  if (PROGRAM_KINDS.has(c.kind)) return single("fixture", "check");
  const args = c.args ?? [];
  if (c.kind === "nvs-suite") {
    const trees = suiteTrees(args);
    if (trees === null) return single("command", "check");
    const cases = ctx.cases.filter((p) => trees.some((t) => p.startsWith(`${t}/`)));
    return { how: "cases", atoms: cases.map(caseId), cases };
  }
  if (c.kind === "cargo-named") {
    let names: string[] | undefined;
    if (args.length === 2 && args[0] === "test" && args[1] === "--workspace") names = ctx.graph ? allTestBinaries(ctx.graph) : [];
    else if (subsetRun(args) !== null && ctx.graph) names = testTargets(ctx.graph, args)?.names;
    if (!names || names.length === 0) return single("command", "check");
    return { how: "tests", atoms: names.map((n) => `test:${n}`), tests: names };
  }
  const argv = c.argv ?? [];
  const cwd = c.cwd ?? ".";
  if (argv[0] === "bun" && argv[1] === "nv" && cwd === ".") {
    const groups = proofGroups(c) ?? runGroups(c);
    if (groups !== null) {
      const dirs = groups.map((g) => ctx.groupDirs.get(g));
      const known = dirs.flatMap((d) => d ?? []);
      const programs = ctx.proofs.filter((p) => known.some((d) => p.startsWith(`${d}/`)));
      const o = own("nv");
      return { how: "proofs", atoms: [o.id, ...programs.map(proofId)], own: o, groups, unknown: ctx.homeless === true || dirs.some((d) => d === undefined) };
    }
    if (argv.length === 3 && argv[2] === "selftest") return { how: "selftest", atoms: ["step:nv", ...ctx.nvTests.map(nvTestId)] };
    return single("nv", "nv");
  }
  if (argv[0] === "bun" && argv[1] === "test" && argv.length === 3 && cwd === "." && ctx.nvTests.includes(argv[2]!.replace(/\\/g, "/"))) {
    const file = argv[2]!.replace(/\\/g, "/");
    return { how: "nvtest", atoms: [nvTestId(file)], file };
  }
  return single("command", "check");
}

/** The groups a `bun nv proofs --run --group ...` check runs whole; null for any other check. */
function runGroups(c: Check): string[] | null {
  const argv = c.argv ?? [];
  if (argv.slice(0, 4).join(" ") !== "bun nv proofs --run") return null;
  const groups: string[] = [];
  for (let i = 4; i < argv.length; i++) {
    if (argv[i] === "--group" && i + 1 < argv.length) groups.push(argv[++i]!);
    else return null;
  }
  return groups.length > 0 ? groups : null;
}

// ---- what a command reads, when nothing in it records ----------------------------------------------

/**
 * The keys a check's command reads that no log records: a `git` command by what it lists or greps, the
 * extension's tests by the extension, and anything else by everything (`*`), since what it reads is
 * not known. `{nvs}` and `bun nv` record their own reads, and are none of these.
 */
export function commandKeys(c: Check): Keyed {
  const argv = c.argv ?? [];
  const keys: Keyed = new Map();
  if (argv[0] === "{nvs}") return keys;
  if (argv[0] === "git") {
    if (argv[1] === "grep" && argv.includes("--")) {
      const specs = argv.slice(argv.indexOf("--") + 1);
      if (specs.length > 0 && specs.every((s) => !/[*?[:]/.test(s))) {
        for (const s of specs) keys.set(`tree:${s.replace(/\\/g, "/").replace(/\/+$/, "") || "."}`, "");
        return keys;
      }
    }
    if (argv[1] === "ls-files" && argv.length > 2 && argv.slice(2).every((a) => !a.startsWith("-"))) {
      for (const p of argv.slice(2)) keys.set(`exists:${p.replace(/\\/g, "/")}`, "");
      return keys;
    }
    for (const k of spawnKeys(argv)) keys.set(k, "");
    return keys;
  }
  if ((argv[0] === "npm" || argv[0] === "npm.cmd") && (c.cwd ?? ".").startsWith("editors/")) {
    keys.set(`tree:${c.cwd}`, "");
    return keys;
  }
  keys.set(WILD, "");
  return keys;
}

// ---- the heavy set's named prediction --------------------------------------------------------------

const TSAN = "tools/tsan.sh";
const FUZZ_MANIFEST = "fuzz/Cargo.toml";
const DB_MATRIX = "tools/nv/cmd/db-matrix.ts";

/** The workspace packages `rel` names as a whole word. */
function namedIn(graph: Graph, rel: string, root: string): string[] {
  let text = "";
  try {
    text = readFileSync(join(root, rel), "utf8");
  } catch {
    return [];
  }
  return [...graph.keys()].filter((n) => new RegExp(`(?<![\\w-])${n.replace(/-/g, "\\-")}(?![\\w-])`).test(text)).sort();
}

/**
 * The workspace packages a heavy check or leg builds, read off the check: `-p <crate>` for a release
 * test, `nvs-cli` for a bench, an `nvs` command or a leg, the packages the fuzz workspace's manifest,
 * the TSan script or the database matrix's module names, and every package for anything else. Each
 * with every package it is compiled against. `extra` is what it reads outside the crates that no log
 * records: the fuzz workspace, the TSan script.
 */
export function heavyCrates(c: Check | null, graph: Graph, root: string = ROOT): { crates: string[]; extra: string[] } {
  const argv = c?.argv ?? [];
  const args = c?.args ?? [];
  const line = argv.join(" ");
  let own: string[];
  let extra: string[] = [];
  if (c === null || argv.includes("{nvs}") || (argv[0] === "bun" && argv[1] === "nv" && argv[2] === "bench")) own = ["nvs-cli"];
  else if (c.kind === "cargo-named" && args.includes("-p")) own = [args[args.indexOf("-p") + 1]!];
  else if (line.includes("fuzz run")) {
    own = namedIn(graph, FUZZ_MANIFEST, root);
    extra = ["fuzz"];
  } else if (line.includes(TSAN)) {
    own = namedIn(graph, TSAN, root);
    extra = [TSAN];
  } else if (argv[0] === "bun" && argv[1] === "nv" && argv[2] === "db-matrix") own = namedIn(graph, DB_MATRIX, root);
  else own = [...graph.keys()];
  if (own.length === 0 || own.some((p) => !graph.has(p))) own = [...graph.keys()];
  const all = new Set(own);
  for (const p of own) for (const d of closure(graph, p, true)) all.add(d);
  return { crates: [...all].sort(), extra };
}

/** `heavyCrates` as keys over `files`, the tree's Rust files: `fn:<file>#*` for every file of those
 * crates, `dir:` for every directory that holds one, and `tree:` for each extra path. */
export function heavyKeys(c: Check | null, graph: Graph, files: string[], root: string = ROOT): Keyed {
  const { crates, extra } = heavyCrates(c, graph, root);
  const dirs = crates.map((p) => graph.get(p)!.dir).filter((d) => d !== "" && d !== ".");
  const keys: Keyed = new Map();
  for (const f of files) {
    if (!dirs.some((d) => f.startsWith(`${d}/`))) continue;
    keys.set(fileWild(f), "");
    const parts = f.split("/");
    for (let i = 1; i < parts.length; i++) keys.set(`dir:${parts.slice(0, i).join("/")}`, "");
  }
  for (const e of extra) keys.set(`tree:${e}`, "");
  // A package at the root builds from every file; so does a graph with no directory to read.
  if (dirs.length < crates.length || keys.size === 0) keys.set(WILD, "");
  return keys;
}

/**
 * What check `c` is made of, as the first lines `bun nv select --explain <check>` prints: how the sweep
 * runs it and which atoms it is, and for a heavy check the crates it builds. Nothing here depends on
 * what changed, so these lines are the same over any tree.
 */
export function describeGroup(c: Check, g: Grouped, graph: Graph | null, root: string = ROOT): string[] {
  const head = `check ${c.id}${c.name ? ` (${c.name})` : ""}:`;
  const own = g.own ? g.own.id : "";
  switch (g.how) {
    case "fixture":
      return [`${head} a fixture, its own atom ${own}, run on the covws nvs and keyed on what that run used`];
    case "cases": {
      const trees = [...new Set((g.cases ?? []).map((p) => p.split("/").slice(0, 2).join("/")))].sort();
      return [`${head} the cases under ${trees.join(", ") || "no tree"}, which nv verify runs too -- ${g.atoms.length} atom(s)`];
    }
    case "tests":
      return [`${head} the test binaries ${(g.tests ?? []).join(", ")}, which nv verify runs too -- ${g.atoms.length} atom(s)`];
    case "proofs":
      return [`${head} its own atom ${own}, keyed on what the command was seen to read, and the proof programs of ${(g.groups ?? []).join(", ")} -- ${g.atoms.length - 1} program(s)${g.unknown ? "; a group has never run, so it runs whole once" : ""}`];
    case "selftest":
      return [`${head} tsc (step:nv) and every tools test file, which nv verify runs too -- ${g.atoms.length} atom(s)`];
    case "nvtest":
      return [`${head} the tools test file ${g.file}, which nv verify runs too`];
    case "nv":
      return [`${head} its own atom ${own}, keyed on what the command was seen to read`];
    case "heavy": {
      const lines = [`${head} a heavy check, its own atom ${own}, held for the floor gate`];
      if (graph) {
        const { crates, extra } = heavyCrates(c, graph, root);
        lines.push(`  it builds ${crates.join(", ")}${extra.length ? `, and reads ${extra.map((e) => (e.includes(".") ? e : `${e}/`)).join(", ")} besides` : ""}: the one prediction the selection keeps`);
      }
      return lines;
    }
    default:
      return [`${head} a command, its own atom ${own}, keyed on what it was seen to use${[...commandKeys(c).keys()].includes(WILD) ? ", and on everything, since nothing records what it reads" : ""}`];
  }
}

/** What a Linux leg holds besides `heavyKeys` for `nvs-cli`: the trees its fixtures and suites read. */
export const LEG_TREES = ["examples", "tests", "docs"];

/** Whether `rel` exists under the root: a named case of a suite must. */
export const onDisk = (rel: string) => existsSync(abs(rel));
