// The live plan's checks as atoms. A plan `[[check]]` is a named group of atoms and its pass
// condition: the check is reached when the selection picks one of its atoms, and a check none of whose
// atoms is picked is not started. What each shape of check is made of:
//
// | check | its atoms | run as |
// |---|---|---|
// | a fixture (`exact`, `ordered`, `contains`, `min-bytes`) | `check:<id>` | `nvs run`, recorded |
// | `nvs-suite`, `nvs test <tree>` | the cases under the tree it names (`case:`) | the picked cases, through `nvs test --cases` |
// | `cargo-named`, `cargo test -p <crate>` narrowed to a target, or `--workspace` | the test binaries it names (`test:`) | each picked binary, whole |
// | `bun nv proofs --verify` or `--run` over groups, or over features (`--only`) | `nv:<id>` and the proof programs of its groups or features (`proof:`), as the last run over each recorded them | the command, which runs only its picked programs |
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
// inside WSL, the database matrix. What each run reads is observed from the footprint log of every
// `nvs` and test binary it starts, and from the reads of a `bun nv` command. What Rust code it runs is
// observed on a **twin** (`heavyTwin`): the same work run again on the covws build, whose verdict counts
// for nothing and whose coverage is the check's keys.
//
// | heavy check | its twin | held besides |
// |---|---|---|
// | a release `cargo test` | the same test binaries and filters, with `--include-ignored`, since a release-only test is ignored in debug | `profile:optimized` |
// | `bun nv bench` on the release CLI | the same bench on the covws `nvs`, one rep, nothing recorded | `profile:optimized` |
// | the database matrix | its SQLite leg (`db-matrix.ts` `sqliteLeg`) | every file of `nvs-db`, whose server drivers only a server leg runs, and `platform:elsewhere` for the socket legs |
// | fuzz, TSan | none: they build inside WSL | `heavyKeys`, every file of the crates they build, and `platform:elsewhere` |
//
// Heavy checks that run the same work have one twin between them (`twinWork`): every release check over
// one test binary and filters, and every database matrix check, runs its twin once per sweep, and each
// records what that run used. A twin that cannot run or be read records `*`, which every change moves and the next run whose twin
// was read drops again (`store.ts` `recordRun`). Fuzz and TSan keep the one prediction left
// in the selection, and it is named as one: `heavyCrates` reads the crates off the fuzz target's source
// file and the TSan script. The two Linux legs run every fixture and suite of the plan inside WSL, so a
// leg is selected whenever the selection picks one of those atoms (`legAtoms`), and its own atom holds
// only `platform:elsewhere` and the modules that run it.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { sqliteLeg } from "../cmd/db-matrix.ts";
import { type Check, heavyRole, isHeavy, PROGRAM_KINDS, proofGroups, subsetRun } from "../driver/accept.ts";
import { allTestBinaries, closure, type Graph, testBinaries } from "../keys/graph.ts";
import { digest } from "../keys/scan.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { caseFiles, caseId, nvTestFiles, nvTestId, proofFiles, proofId } from "./atoms.ts";
import { fileWild, PLATFORM_ONLY, PROFILE_ONLY, spawnKeys, WILD } from "./keys.ts";
import { featureGroup, type Keyed, proofReadsSlot, type SelectStore } from "./store.ts";

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
  /** `proofs` over named features (`--only`): the features. */
  features?: string[];
  /** `proofs`: a group or feature `nv proofs` has never recorded, or a new program no recorded directory
   * holds, so which programs it runs is not known yet. */
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
  if (isHeavy(c)) {
    // A run adds keys to a footprint and never removes one while the definition stays the same, so what
    // a heavy check's keys are read from, its twin or its predicted crates, is part of its definition:
    // when that changes, the check runs once more and records the new keys in place of the old ones.
    const def = ctx.graph ? digest(`${checkDef(c)} ${JSON.stringify(heavyBasis(c, ctx.graph))}`) : checkDef(c);
    return { how: "heavy", atoms: [`heavy:${c.id}`], own: { id: `heavy:${c.id}`, def } };
  }
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
    const features = groups === null ? onlyFeatures(c) : null;
    if (groups !== null || features !== null) {
      const dirs = groups !== null ? groups.map((g) => ctx.groupDirs.get(g)) : features!.map((f) => ctx.groupDirs.get(featureGroup(f)));
      const known = dirs.flatMap((d) => d ?? []);
      const programs = ctx.proofs.filter((p) => known.some((d) => p.startsWith(`${d}/`)));
      const o = own("nv");
      const unknown = ctx.homeless === true || dirs.some((d) => d === undefined);
      return { how: "proofs", atoms: [o.id, ...programs.map(proofId)], own: o, ...(groups !== null ? { groups } : { features: features! }), unknown };
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

/** The features a `bun nv proofs --verify --only ...` or `--run --only ...` check runs the programs of;
 * null for any other check. */
function onlyFeatures(c: Check): string[] | null {
  const argv = c.argv ?? [];
  if (c.kind !== "command" || argv.slice(0, 3).join(" ") !== "bun nv proofs" || (argv[3] !== "--verify" && argv[3] !== "--run") || argv[4] !== "--only") return null;
  const ids = argv.slice(5);
  return ids.length > 0 && ids.every((a) => !a.startsWith("-")) ? ids : null;
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
const FUZZ_TARGETS = "fuzz/fuzz_targets";
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

/** The fuzz target a `cargo fuzz run <target>` command line names, or "" for none. */
export function fuzzTarget(line: string): string {
  return /fuzz run\s+([\w-]+)/.exec(line)?.[1] ?? "";
}

/**
 * The workspace packages the Rust source `rel` uses in its code, as `nvs_syntax`-style paths. Whole-line
 * and block comments are left out, because a doc comment names crates the file never links; a trailing
 * comment is kept, which can only add a package. `null` when the file cannot be read.
 */
export function usedIn(graph: Graph, rel: string, root: string): string[] | null {
  let text: string;
  try {
    text = readFileSync(join(root, rel), "utf8");
  } catch {
    return null;
  }
  const code = text
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .filter((l) => !l.trimStart().startsWith("//"))
    .join("\n");
  return [...graph.keys()].filter((n) => new RegExp(`(?<![\\w])${n.replace(/-/g, "_")}(?![\\w])`).test(code)).sort();
}

/** One run of a twin: a test binary of the covws build (`<package> <kind> <target>`) and the harness
 * arguments it takes. */
export interface TwinTest {
  name: string;
  args: string[];
}

/** What a heavy check runs again on the covws build to learn which Rust code its own run used. */
export interface Twin {
  /** Test binaries to run, each with its harness arguments. */
  tests: TwinTest[];
  /** A command to run, `{nvs}` standing for the covws `nvs`; empty for none. */
  argv: string[];
  /** The environment the twin's processes get besides the recorder's; `{scratch}` in a value stands for
   * a fresh directory. */
  env: Record<string, string>;
  /** Keys held besides what the twin records, for code the twin cannot reach. */
  held: string[];
  /** Packages whose every file is held, for code the twin cannot reach. */
  crates: string[];
}

/** Cargo options that take a value, which is therefore no test-name filter. */
const VALUED = new Set(["-p", "--package", "--bin", "--test", "--example", "--bench", "--features", "-F", "--target", "--profile", "-j", "--jobs", "--manifest-path"]);

/** The arguments `cargo <args>` hands each test binary: its test-name filters and what follows `--`. */
export function harnessArgs(args: string[]): string[] {
  const at = args.indexOf("--");
  const head = at < 0 ? args : args.slice(0, at);
  const filters: string[] = [];
  for (let i = head[0] === "test" ? 1 : 0; i < head.length; i++) {
    const a = head[i]!;
    if (VALUED.has(a)) i++;
    else if (!a.startsWith("-")) filters.push(a);
  }
  return [...filters, ...(at < 0 ? [] : args.slice(at + 1))];
}

/** The test binaries `cargo <args>` runs: `testTargets` for a `-p` run, and every package's target of
 * that name for a `--bin` or `--test` run that names no package. */
export function suiteBinaries(graph: Graph, args: string[]): string[] {
  const named = testTargets(graph, args);
  if (named) return named.names;
  const flag = (f: string) => (args.includes(f) ? args[args.indexOf(f) + 1] : undefined);
  const bin = flag("--bin");
  const test = flag("--test");
  const out: string[] = [];
  for (const pkg of [...graph.keys()].sort()) {
    for (const b of testBinaries(graph, pkg)) {
      if ((bin !== undefined && b.target.kind === "bin" && b.target.name === bin) || (test !== undefined && b.target.kind === "test" && b.target.name === test)) out.push(b.name);
    }
  }
  return out;
}

/** `bun nv bench` as its twin runs it: on the covws `nvs`, one rep and few requests, with no budget to
 * fail and nothing written. */
export function benchTwin(argv: string[]): string[] {
  const valued = new Set(["--record", "--max-work-ms", "--max-ms", "--nvs", "--reps", "--json", "--requests"]);
  const out: string[] = [];
  for (let i = 0; i < argv.length; i++) {
    if (valued.has(argv[i]!)) i++;
    else if (argv[i] !== "--check") out.push(argv[i]!);
  }
  out.push("--nvs", "{nvs}", "--allow-debug", "--reps", "1");
  if (out.includes("--serve-vs-fpm")) out.push("--requests", "200");
  return out;
}

/** The twin of heavy check `c`, or null for one that has none: fuzz, TSan, and a check that runs on the
 * covws build already. The module doc's table says what each twin is. */
export function heavyTwin(c: Check, graph: Graph): Twin | null {
  const args = c.args ?? [];
  const argv = c.argv ?? [];
  if (c.kind === "cargo-named" && args[0] === "test" && args.includes("--release")) {
    const names = suiteBinaries(graph, args.filter((a) => a !== "--release"));
    if (names.length === 0) return null;
    const run = [...harnessArgs(args), "--include-ignored"];
    return { tests: names.map((name) => ({ name, args: run })), argv: [], env: {}, held: [PROFILE_ONLY], crates: [] };
  }
  if (argv[0] === "bun" && argv[1] === "nv" && argv[2] === "bench") return { tests: [], argv: benchTwin(argv), env: {}, held: [PROFILE_ONLY], crates: [] };
  if (heavyRole(c) === "db-matrix") {
    const leg = sqliteLeg();
    const tests = leg.suites.flatMap((s) => suiteBinaries(graph, ["test", ...s]).map((name) => ({ name, args: harnessArgs(["test", ...s]) })));
    return { tests, argv: [], env: leg.env, held: [PLATFORM_ONLY, "tree:tests/db"], crates: ["nvs-db"] };
  }
  return null;
}

/** For each heavy check of `plan` whose twin runs test binaries, its atom and those binaries, as
 * `QueryOptions.ran` takes them. */
export function twinBinaries(plan: Check[], graph: Graph | null): Map<string, string[]> {
  const out = new Map<string, string[]>();
  if (!graph) return out;
  for (const c of plan) {
    const twin = isHeavy(c) ? heavyTwin(c, graph) : null;
    if (twin && twin.tests.length > 0) out.set(`heavy:${c.id}`, twin.tests.map((t) => t.name));
  }
  return out;
}

/** The work `twin` runs, as a key: two twins with the same test binaries and arguments, command and
 * environment run the same processes, so a sweep runs them once. What it holds besides is left out, since
 * each check adds its own to what the run recorded. */
export function twinWork(twin: Twin): string {
  return JSON.stringify([twin.tests.map((t) => [t.name, t.args]), twin.argv, Object.entries(twin.env).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))]);
}

/** Whether heavy check `c` is keyed on a prediction: fuzz and TSan, which build and run inside WSL. */
export function predicted(c: Check): boolean {
  const role = heavyRole(c);
  return role === "fuzz" || role === "tsan";
}

/** What a heavy check's keys are read from, which is part of its definition: its twin, its predicted
 * crates, or its own run on the covws build. */
function heavyBasis(c: Check, graph: Graph): unknown {
  if (predicted(c)) return { predicted: heavyCrates(c, graph) };
  const twin = heavyTwin(c, graph);
  return twin ? { twin } : { observed: true };
}

/**
 * The atoms a Linux leg runs again on Linux: every fixture of the plan and every case of its suites. A
 * leg is selected whenever the selection picks one of them.
 */
export function legAtoms(plan: Check[], groups: Map<string, Grouped>): string[] {
  const out: string[] = [];
  for (const c of plan) {
    if (!PROGRAM_KINDS.has(c.kind) && c.kind !== "nvs-suite") continue;
    out.push(...(groups.get(c.id)?.atoms ?? []));
  }
  return out;
}

/** What a leg's own atom holds: code only Linux compiles, and the modules that run the legs. */
export const LEG_KEYS = [PLATFORM_ONLY, "mod:tools/nv/driver/legs.ts", "mod:tools/nv/driver/mirror.ts"];

/**
 * The workspace packages a heavy check builds: what fuzz and TSan are keyed on, and what a check whose
 * twin could not be read widens to. Read off the check: `-p <crate>` for a release
 * test, `nvs-cli` for a bench, an `nvs` command or a leg, the packages a fuzz target's own source file
 * uses (the fuzz workspace's manifest when that file cannot be read), the packages the TSan script or
 * the database matrix's module names, and every package for anything else. Each
 * with every package it is compiled against. `extra` is what it reads outside the crates that no log
 * records: the fuzz workspace, the TSan script.
 */
export function heavyCrates(c: Check | null, graph: Graph, root: string = ROOT): { crates: string[]; extra: string[] } {
  const argv = c?.argv ?? [];
  const args = c?.args ?? [];
  const line = argv.join(" ");
  let own: string[];
  let extra: string[] = [];
  // A fuzz target links its packages as ordinary dependencies, so their dev-dependencies are not built.
  let dev = true;
  if (c === null || argv.includes("{nvs}") || (argv[0] === "bun" && argv[1] === "nv" && argv[2] === "bench")) own = ["nvs-cli"];
  else if (c.kind === "cargo-named" && args.includes("-p")) own = [args[args.indexOf("-p") + 1]!];
  else if (line.includes("fuzz run")) {
    const target = fuzzTarget(line);
    own = (target && usedIn(graph, `${FUZZ_TARGETS}/${target}.rs`, root)) || namedIn(graph, FUZZ_MANIFEST, root);
    extra = ["fuzz"];
    dev = false;
  } else if (line.includes(TSAN)) {
    own = namedIn(graph, TSAN, root);
    extra = [TSAN];
  } else if (argv[0] === "bun" && argv[1] === "nv" && argv[2] === "db-matrix") own = namedIn(graph, DB_MATRIX, root);
  else own = [...graph.keys()];
  if (own.length === 0 || own.some((p) => !graph.has(p))) own = [...graph.keys()];
  const all = new Set(own);
  for (const p of own) for (const d of closure(graph, p, dev)) all.add(d);
  return { crates: [...all].sort(), extra };
}

/** `heavyCrates` as keys over `files`, the tree's Rust files: `fn:<file>#*` for every file of those
 * crates, `dir:` for every directory that holds one, and `tree:` for each extra path. */
export function heavyKeys(c: Check | null, graph: Graph, files: string[], root: string = ROOT): Keyed {
  const { crates, extra } = heavyCrates(c, graph, root);
  return crateKeys(crates, extra, graph, files);
}

/** Packages `crates` as keys over `files`, the tree's Rust files: `fn:<file>#*` for every file of them,
 * `dir:` for every directory that holds one, and `tree:` for each of `extra`. */
export function crateKeys(crates: string[], extra: string[], graph: Graph, files: string[]): Keyed {
  const dirs = crates.flatMap((p) => graph.get(p)?.dir ?? []).filter((d) => d !== "" && d !== ".");
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
      return [`${head} its own atom ${own}, keyed on what the command was seen to read, and the proof programs of ${(g.groups ?? g.features ?? []).join(", ")} -- ${g.atoms.length - 1} program(s)${g.unknown ? "; a group or feature has never run, so it runs whole once" : ""}`];
    case "selftest":
      return [`${head} tsc (step:nv) and every tools test file, which nv verify runs too -- ${g.atoms.length} atom(s)`];
    case "nvtest":
      return [`${head} the tools test file ${g.file}, which nv verify runs too`];
    case "nv":
      return [`${head} its own atom ${own}, keyed on what the command was seen to read`];
    case "heavy": {
      const lines = [`${head} a heavy check, its own atom ${own}, held for the floor gate`];
      const twin = graph && !predicted(c) ? heavyTwin(c, graph) : null;
      if (graph && predicted(c)) {
        const { crates, extra } = heavyCrates(c, graph, root);
        lines.push(`  it builds ${crates.join(", ")}${extra.length ? `, and reads ${extra.map((e) => (e.includes(".") ? e : `${e}/`)).join(", ")} besides` : ""}: the one prediction the selection keeps`);
      } else if (twin) {
        const runs = [...twin.tests.map((t) => `${t.name}${t.args.length ? ` ${t.args.join(" ")}` : ""}`), ...(twin.argv.length ? [twin.argv.join(" ")] : [])];
        const held = [...twin.held, ...twin.crates.map((p) => `every file of ${p}`)];
        lines.push(`  keyed on its twin on the covws build: ${runs.join("; ")}${held.length ? `; held besides: ${held.join(", ")}` : ""}`);
      } else if (graph) lines.push("  keyed on what its own run on the covws build used");
      return lines;
    }
    default:
      return [`${head} a command, its own atom ${own}, keyed on what it was seen to use${[...commandKeys(c).keys()].includes(WILD) ? ", and on everything, since nothing records what it reads" : ""}`];
  }
}

/** Whether `rel` exists under the root: a named case of a suite must. */
export const onDisk = (rel: string) => existsSync(abs(rel));
