// `bun nv proofs`: what every feature Novis ships still owes of its feature proofs.
//
//     bun nv proofs                        one line per group: how many of its features have each proof
//     bun nv proofs --group 'Core\Str'     one line per feature of that group
//     bun nv proofs --id 'Core\Str::length'  one feature: every proof it has, and what it still owes
//     bun nv proofs --owed [--limit N]     one line per feature still owing a proof: the worklist
//     bun nv proofs --gaps                 every proof carrying a `proof: gap` marker
//     bun nv proofs --gate                 exit 0 when nothing in scope is owed; name what is otherwise
//     bun nv proofs --json                 the audit as JSON
//     bun nv proofs --run                  run every example and attack in scope, and report what failed
//     bun nv proofs --verify               --gate, then --run, for each scope in one pass
//     bun nv proofs --run --id ID --show   first each of the feature's programs as it runs: a `== <path>`
//                                          line, its output, its exit status and time
//     bun nv proofs --bless FILE...        write each example's `.out` from what it prints, and show it
//     bun nv proofs --comments PATH...     judge the comments of these programs, or of every `.nvs` under
//                                          a directory, against the plain-comment bounds; runs nothing
//     bun nv proofs --record-perf          measure every bench in scope with no current figure, and append
//                                          a record per feature to the perf ledger
//     bun nv proofs --perf-report          write docs/perf/members.md from the ledger
//
// `--group` and `--only ID...` narrow the scope of `--owed`, `--gaps`, `--gate`, `--json`, `--run`,
// `--verify` and `--record-perf`, and `--id` narrows `--run`, `--verify` and `--record-perf`. `--run`,
// `--verify` and `--record-perf` take `--group` more than once: the roster is read once, every program
// runs in one pool, and each group prints its own verdict lines under a `== <group>` line and closes
// on `-- <group>: passed` or `-- <group>: failed`, which the loop driver splits on. `--no-perf`
// stops the perf proof from being owed. `--record-perf` takes `--reps N` timed runs per program (5),
// `--force` to re-measure what already has a current figure, `--note` to record a word with each record,
// and `--perf-report` to write the report after it. `--nvs` names the binary to use as it is. Without it,
// the audit reads the roster from `target/release` and then `target/debug`, `--bless`, `--run` and
// `--verify` use the proof binary, and `--record-perf` uses the release binary, each built first when it
// is not current.
// `rule:testing/feature-proofs` is what a feature owes, `tools/nv/proofs/roster.ts` is where the features
// come from, `tools/nv/proofs/collect.ts` is how each proof is found on disk, `tools/nv/proofs/run.ts` is
// how a proof program is run and judged, and `tools/nv/proofs/perf.ts` is how a figure is taken.
//
// This replaces the Python proof tool's audit, `--run`, `--verify`, `--bless`, `--comments`,
// `--record-perf` and `--perf-report`.

import { existsSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { abs, rel } from "../lib/paths.ts";
import { ArgError, comparePaths, fixed, parseArgs, pyInt, pyRepr } from "../lib/py.ts";
import { collect, commentProblems, gapTitle, HELP_BACKLOG_REASON, implHash, knownGap, loadPolicy, owed, PROOFS, shownProofs, walk, type Policy, type Proof, type Proofs, type Skips } from "../proofs/collect.ts";
import { perfReport, recordPerf } from "../proofs/perf.ts";
import { aboutFile, benchFile, examplesDir, hostileDir, namesIn, read, roster, RosterError, type Entry } from "../proofs/roster.ts";
import { bless, namedBinary, proofBinary, releaseBinary, runPrograms, saveReads, showProgram, suiteLines, type Binary, type What } from "../proofs/run.ts";

export const summary =
  "what each feature still owes of its proofs, and whether they pass: nv proofs [--group G]... [--only ID...] [--id ID] [--owed] [--gaps] [--gate] [--json] [--run] [--verify] [--bless FILE...] [--comments PATH...] [--record-perf] [--perf-report]";

const USAGE = [
  "usage: nv proofs [-h] [--group GROUP] [--only ID [ID ...]] [--id FEATURE]",
  "                 [--owed] [--gaps] [--limit LIMIT] [--json] [--gate]",
  "                 [--run] [--verify] [--valgrind] [--quiet] [--no-cache]",
  "                 [--strict] [--show] [--no-perf] [--nvs NVS]",
  "                 [--bless FILE [FILE ...]] [--comments PATH [PATH ...]]",
  "                 [--record-perf] [--reps REPS] [--force] [--note NOTE]",
  "                 [--perf-report] [--impl-hash FILE [FILE ...]]",
].join("\n");

/** What the binary is: the one named, or the newest profile built. */
function binary(explicit: string | undefined): string | null {
  if (explicit !== undefined) return existsSync(explicit) ? explicit : null;
  const exe = process.platform === "win32" ? "nvs.exe" : "nvs";
  for (const profile of ["release", "debug"]) {
    const p = abs(`target/${profile}/${exe}`);
    if (existsSync(p)) return p;
  }
  return null;
}

const ljust = (s: string, n: number) => s + " ".repeat(Math.max(0, n - [...s].length));
const rjust = (s: string | number, n: number) => " ".repeat(Math.max(0, n - [...String(s)].length)) + String(s);

/** A float from the ledger as Python's `str()` prints it. */
const pyFloat = (v: unknown) => (typeof v === "number" && Number.isInteger(v) ? `${v}.0` : String(v));

/** JSON as Python's `json.dumps(..., indent=2)` writes it, non-ASCII escaped. */
function pythonJson(value: unknown): string {
  return JSON.stringify(value, null, 2).replace(/[\u0080-￿]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`);
}

const HEADINGS: Record<Proof, string> = { tests: "tests", examples: "exmpl", perf: "perf", hostile: "hostl", about: "about", help: "help" };

interface Row {
  group: string;
  kind: string;
  features: number;
  complete: number;
  gaps: number;
  counts: Record<Proof, number>;
}

/** One row per group, least complete first. */
function groupRows(entries: Entry[], proofs: Map<string, Proofs>, policy: Policy, skips: Skips): Row[] {
  const groups = new Map<string, Entry[]>();
  for (const e of entries) {
    if (!groups.has(e.group)) groups.set(e.group, []);
    groups.get(e.group)!.push(e);
  }
  const rows: Row[] = [];
  for (const [group, members] of groups) {
    const counts = Object.fromEntries(PROOFS.map((p) => [p, 0])) as Record<Proof, number>;
    let complete = 0;
    for (const e of members) {
      const missing = owed(e, proofs.get(e.id)!, policy, skips);
      for (const p of PROOFS) if (!(p in missing)) counts[p]++;
      if (Object.keys(missing).length === 0) complete++;
    }
    const gaps = members.reduce((n, e) => n + proofs.get(e.id)!.gaps.length, 0);
    rows.push({ group, kind: members[0]!.kind, features: members.length, complete, gaps, counts });
  }
  return rows.sort((a, b) => a.complete / a.features - b.complete / b.features || b.features - a.features || (a.group < b.group ? -1 : a.group > b.group ? 1 : 0));
}

function printStatus(out: string[], rows: Row[], columns: Proof[]): void {
  const total = rows.reduce((n, r) => n + r.features, 0);
  const done = rows.reduce((n, r) => n + r.complete, 0);
  out.push(`== WHAT EACH FEATURE STILL OWES  (${total} features in ${rows.length} groups, ${done} complete)`);
  out.push("-- a column counts the features of that group whose proof is on disk and current.");
  out.push("-- `bun nv proofs --group <name>` opens one; --owed is the worklist.");
  if (!columns.includes("perf")) out.push("-- the perf proof is switched off, so it is neither owed nor shown.");
  out.push("");
  out.push(`  ${rjust("DONE", 9)}  ` + columns.map((c) => rjust(HEADINGS[c], 6)).join(" ") + "  group");
  for (const r of rows) {
    out.push(`  ${rjust(r.complete, 4)}/${ljust(String(r.features), 4)}  ` + columns.map((c) => rjust(r.counts[c], 6)).join(" ") + `  ${r.group}`);
  }
  out.push("");
  out.push(total ? `  ${done}/${total} features complete (${fixed((done / total) * 100, 1)}%)` : "  nothing on the roster");
  const gaps = rows.reduce((n, r) => n + r.gaps, 0);
  if (gaps) {
    out.push(`  ${gaps} proof(s) carry a \`proof: gap\` marker: a bug the proof found and nobody has`);
    out.push("  fixed yet, recorded as a gap under `data/gaps/`. `--run … --strict` fails");
    out.push("  on them; `--owed --gaps` lists them. This number going up is the point of the");
    out.push("  hostile tree, and it going down is the point of the rest of the repository.");
  }
}

function printGroup(out: string[], name: string, entries: Entry[], proofs: Map<string, Proofs>, policy: Policy, skips: Skips, columns: Proof[]): void {
  const members = entries.filter((e) => e.group === name).sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  out.push(`== ${name}  (${members.length} features)`);
  out.push("  " + columns.map((c) => rjust(HEADINGS[c], 5)).join(" ") + "  feature");
  for (const e of members) {
    const p = proofs.get(e.id)!;
    const missing = owed(e, p, policy, skips);
    const have: Record<Proof, number> = {
      tests: p.nvst.length + p.rust.length,
      examples: p.examples.length,
      perf: p.perf ? 1 : 0,
      hostile: p.hostile.length,
      about: p.about ? 1 : 0,
      help: e.help ? 0 : 1,
    };
    const skipped = skips.get(e.id) ?? {};
    const cells = columns.map((c) => rjust(c in skipped ? "-" : `${have[c]}${c in missing ? "!" : ""}`, 5));
    out.push("  " + cells.join(" ") + `  ${e.id}`);
  }
  out.push("\n  `!` marks a proof still owed, `-` one this feature is excused from.");
}

function printEntry(out: string[], fid: string, entries: Entry[], proofs: Map<string, Proofs>, policy: Policy, skips: Skips): number {
  const match = entries.find((e) => e.id === fid);
  if (!match) {
    const near = entries.filter((e) => e.id.toLowerCase().includes(fid.toLowerCase())).slice(0, 8).map((e) => e.id);
    out.push(`nv proofs: no feature ${pyRepr(fid)}.` + (near.length ? ` Near: ${near.join(", ")}` : ""));
    return 1;
  }
  const p = proofs.get(match.id)!;
  const missing = owed(match, p, policy, skips);
  out.push(`== ${match.id}   [${match.kind}]`);
  if (match.summary) out.push(`   ${[...match.summary].slice(0, 100).join("")}`);
  if (match.anchor) out.push(`   implemented at ${match.anchor}`);
  if (match.twin.length) out.push(`   replaces PHP: ${match.twin.join(", ")}  (a differential case needs no frozen output)`);
  out.push("");
  out.push(`   about     ${p.about || "none at " + aboutFile(match)}`);
  out.push(`   tests     ${p.nvst.length} case(s), ${p.rust.length} Rust`);
  for (const f of p.nvst.slice(0, 6)) out.push(`     ${f}`);
  if (p.inferred) out.push(`     (${p.inferred} credited by a call rather than a \`covers:\` marker)`);
  for (const f of p.rust.slice(0, 6)) out.push(`     ${f}`);
  out.push(`   examples  ${p.examples.length} in ${examplesDir(match)}`);
  for (const f of p.examples) out.push(`     ${f}`);
  out.push(`   perf      ${p.bench || "no bench at " + benchFile(match)}`);
  if (p.perf) {
    out.push(`     ${pyFloat(p.perf.ns_per_op)} ns/op, ${pyFloat(p.perf.ratio)} units, measured at ${p.perf.commit} on ${p.perf.machine}`);
  }
  const counted = p.perfAny && "allocations" in p.perfAny ? p.perfAny : p.perf && "allocations" in p.perf ? p.perf : null;
  if (counted) {
    const expected = counted.expected as Record<string, unknown> | undefined;
    const declares = expected && Object.keys(expected).length ? `  (declares ${Object.entries(expected).map(([k, v]) => `${k} ${v}`).join(", ")})` : "";
    const n = (k: string, d: number) => fixed(counted[k] as number, d);
    out.push(`     per op: ${n("statements", 2)} statements, ${n("calls", 2)} calls, ${n("allocations", 2)} allocations, ${n("bytes", 1)} bytes${declares}`);
  }
  out.push(`   hostile   ${p.hostile.length} in ${hostileDir(match)}`);
  for (const f of p.hostile) out.push(`     ${f}`);
  if (p.gaps.length) {
    out.push(`   known-gap ${p.gaps.length} proof(s) found a bug nobody has fixed:`);
    for (const f of p.gaps) out.push(`     ${f} -> ${gapLine(f)}`);
  }
  out.push("");
  if (Object.keys(missing).length) {
    out.push("   OWED:");
    for (const [proof, why] of Object.entries(missing)) out.push(`     ${ljust(proof, 9)} ${why}`);
  } else {
    out.push("   complete.");
  }
  return 0;
}

/** `<gap id>: <its title>` for the proof at `path`, as `--id` and `--gaps` print it. */
function gapLine(path: string): string {
  const id = knownGap(read(path));
  return id ? `${id}: ${gapTitle(id) ?? ""}` : "?: ";
}

function printOwed(out: string[], scope: Entry[], proofs: Map<string, Proofs>, policy: Policy, skips: Skips, limit: number): void {
  const rows = scope.map((e) => [e, owed(e, proofs.get(e.id)!, policy, skips)] as const).filter(([, m]) => Object.keys(m).length);
  out.push(`== STILL OWED  (${rows.length} features)`);
  out.push("-- one line per feature; a session takes a feature, not a column, because all its proofs\n-- spend the same understanding of what the feature does at its edges.");
  out.push("");
  for (const [e, missing] of rows.slice(0, limit)) out.push(`  ${ljust(e.id, 46)} ${Object.keys(missing).sort().join(", ")}`);
  if (rows.length > limit) out.push(`  ... and ${rows.length - limit} more (--limit 0 for all)`);
}

/** Exit 0 when nothing in scope is owed. It reads the trees and the ledger and runs nothing. */
function gate(out: string[], scope: Entry[], proofs: Map<string, Proofs>, policy: Policy, skips: Skips, label: string | null): number {
  const missing = scope.map((e) => [e, owed(e, proofs.get(e.id)!, policy, skips)] as const).filter(([, m]) => Object.keys(m).length);
  const where = label ?? "the whole roster";
  if (missing.length) {
    out.push(`nv proofs gate: ${missing.length} of ${scope.length} features in ${where} still owe a proof.`);
    for (const [e, m] of missing.slice(0, 30)) {
      out.push(`  ${ljust(e.id, 46)} ${Object.keys(m).sort().map((k) => `${k}: ${m[k]}`).join(", ")}`);
    }
    if (missing.length > 30) out.push(`  ... and ${missing.length - 30} more`);
    return 1;
  }
  const plain = Object.values(policy).some((k) => k.comments) ? ", plain comments" : "";
  out.push(`nv proofs gate: nothing owed in ${where} (${scope.length} features, each owing ${shownProofs(policy).join(", ")}${plain}).`);
  if (label === null) {
    const backlog = [...skips.values()].filter((r) => r.help === HELP_BACKLOG_REASON).length;
    out.push(backlog ? `help backlog: ${backlog} feature(s) skip help until they have it` : "help backlog: empty");
  }
  return 0;
}

/** `--only`'s words, taken out of `args` the way argparse's `nargs="+"` takes them. */
function takeList(args: string[], flag: string): { rest: string[]; list: string[] | null } {
  const rest: string[] = [];
  let list: string[] | null = null;
  for (let i = 0; i < args.length; i++) {
    if (args[i] !== flag) {
      rest.push(args[i]!);
      continue;
    }
    list = [];
    while (i + 1 < args.length && !args[i + 1]!.startsWith("-")) list.push(args[++i]!);
    if (list.length === 0) throw new ArgError(`argument ${flag}: expected at least one argument`);
  }
  return { rest, list };
}

/** A path as the user wrote it, from the working directory, made repository-relative. */
const repoPath = (p: string) => rel(resolve(p));

/** `--comments`: judges the named programs, or every `.nvs` under a named directory, against the
 * bounds of AGENTS.md § *Text an end user reads*. It reads and never runs anything. */
function checkComments(out: string[], targets: string[]): number {
  const files = [...new Set(targets.map(repoPath).flatMap((t) => (statSync(abs(t), { throwIfNoEntry: false })?.isDirectory() ? walk(t, ".nvs") : [t])))].sort(comparePaths);
  let bad = 0;
  for (const path of files) {
    const problems = commentProblems(path);
    if (problems.length === 0) continue;
    bad++;
    out.push(`  ${path}`, ...problems.map((p) => `      ${p}`));
  }
  const rule = "`Text an end user reads` in AGENTS.md";
  if (bad) {
    out.push(`nv proofs comments: ${bad} of ${files.length} program(s) miss the bounds of ${rule}.`);
    return 1;
  }
  out.push(`nv proofs comments: ${files.length} program(s), each inside the bounds of ${rule}.`);
  return 0;
}

/** Every `--group` value in order, since `--run` and `--verify` take several. */
function groupsIn(args: string[]): string[] {
  const groups: string[] = [];
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--group" && i + 1 < args.length) groups.push(args[++i]!);
    else if (args[i]!.startsWith("--group=")) groups.push(args[i]!.slice("--group=".length));
  }
  return groups;
}

/** A scope whose verdict lines print together: a group, one feature, the named features or the roster. */
interface Scope {
  label: string | null;
  entries: Entry[];
}

/** The programs a scope runs: the `.nvs` files directly in each feature's own directory, which is the set
 * `owed` counts. A `.nvs` in a subdirectory is material a case loads and does not run on its own. */
function programsOf(scope: Scope, what: What): string[] {
  const dirs = scope.entries.map((e) => (what === "examples" ? examplesDir(e) : hostileDir(e)));
  return [...new Set(dirs.flatMap((d) => namesIn(d, ".nvs").map((n) => `${d}/${n}`)))].sort();
}

/** A group's own paths, which its `proofs: <group>` unit keys on: each feature's example and attack
 * directories, read whole, and its bench file. */
function groupReads(entries: Entry[]): string[] {
  return [...new Set(entries.flatMap((e) => [examplesDir(e), hostileDir(e), benchFile(e)]))].sort();
}

/** `--run`, or `--verify` when `verify`: each scope's gate first when verifying, then both suites over
 * every scope whose gate passed, in one pool. */
async function runScopes(out: string[], bin: Binary, scopes: Scope[], verify: boolean, flags: Set<string>, proofs: Map<string, Proofs>, policy: Policy, skips: Skips): Promise<number> {
  const opts = { valgrind: flags.has("--valgrind"), cache: !flags.has("--no-cache"), strict: flags.has("--strict"), quiet: flags.has("--quiet") };
  const heads = new Map<Scope, string[]>();
  const running: Scope[] = [];
  let rc = 0;
  for (const scope of scopes) {
    const lines: string[] = [];
    if (scopes.length > 1) lines.push(`== ${scope.label ?? "the whole roster"}`);
    const gated = verify ? gate(lines, scope.entries, proofs, policy, skips, scope.label) : 0;
    rc |= gated;
    heads.set(scope, lines);
    if (!gated) running.push(scope);
  }
  const suites: What[] = ["examples", "hostile"];
  if (flags.has("--show")) {
    // One program at a time, printed as it finishes, in the order examples, attacks, bench.
    for (const scope of running) {
      for (const what of suites) for (const path of programsOf(scope, what)) process.stdout.write(await showProgram(bin.path, path, what));
      for (const e of scope.entries) if (existsSync(abs(benchFile(e)))) process.stdout.write(await showProgram(bin.path, benchFile(e), "bench"));
    }
  }
  const programs = running.flatMap((s) => suites.flatMap((what) => programsOf(s, what).map((path) => ({ what, path }))));
  const pass = await runPrograms(bin, programs, opts);
  for (const scope of scopes) {
    const lines = heads.get(scope)!;
    let failed = !running.includes(scope);
    if (!failed) for (const what of suites) if (suiteLines(lines, what, programsOf(scope, what), pass, opts)) failed = true;
    // Several scopes close each section on its verdict, which is what the driver hands each check.
    if (scopes.length > 1) lines.push(`-- ${scope.label ?? "the whole roster"}: ${failed ? "failed" : "passed"}`);
    out.push(...lines);
    if (failed) rc = 1;
  }
  return rc;
}

export async function run(args: string[]): Promise<number> {
  let flags: Set<string>;
  let values: Map<string, string>;
  let only: string[] | null;
  let comments: string[] | null;
  let blessed: string[] | null;
  let hashed: string[] | null;
  let limit = 40;
  let reps = 5;
  try {
    let rest: string[];
    ({ rest, list: only } = takeList(args, "--only"));
    ({ rest, list: comments } = takeList(rest, "--comments"));
    ({ rest, list: blessed } = takeList(rest, "--bless"));
    ({ rest, list: hashed } = takeList(rest, "--impl-hash"));
    ({ flags, values } = parseArgs(rest, {
      flags: ["--owed", "--gaps", "--json", "--gate", "--no-perf", "--run", "--verify", "--valgrind", "--quiet", "--no-cache", "--strict", "--show", "--record-perf", "--force", "--perf-report"],
      valued: ["--group", "--id", "--limit", "--nvs", "--reps", "--note"],
    }));
    const int = (flag: string) => {
      const raw = values.get(flag);
      if (raw === undefined) return undefined;
      const n = pyInt(raw);
      if (n === null) throw new ArgError(`argument ${flag}: invalid int value: ${pyRepr(raw)}`);
      return n;
    };
    limit = int("--limit") ?? limit;
    reps = int("--reps") ?? reps;
    if (reps < 1) throw new ArgError("argument --reps: must be at least 1");
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv proofs: error: ${e.message}`);
    return 2;
  }
  if (flags.has("--help")) {
    console.log(`${USAGE}\n\nnv proofs: ${summary}`);
    return 0;
  }
  const out: string[] = [];
  const flush = (code: number) => {
    if (out.length > 0) process.stdout.write(out.join("\n") + "\n");
    return code;
  };

  // Only what runs a proof program pays for a current binary. The audit reads a roster, and a person
  // asking `--owed` is not made to wait for a build.
  if (comments !== null) return flush(checkComments(out, comments));
  // What a perf record's `impl_hash` is for each file, one `<hash> <path>` line each. The Python proof
  // tool reads its currency from here, so the two tools cannot disagree about it.
  if (hashed !== null) {
    for (const path of hashed) out.push(`${implHash(path) || "-"} ${path}`);
    return flush(0);
  }
  const measures = flags.has("--record-perf");
  if (flags.has("--perf-report") && !measures) return flush(perfReport(out));
  const executes = blessed !== null || flags.has("--run") || flags.has("--verify");
  let bin: Binary | null = null;
  const named = values.get("--nvs");
  if ((executes || measures) && named === undefined) {
    const built = await (measures ? releaseBinary() : proofBinary());
    if (typeof built === "string") {
      out.push(`nv proofs: ${built}`);
      return flush(1);
    }
    bin = built;
  }
  const nvs = bin?.path ?? binary(named);
  if (nvs === null) {
    out.push("nv proofs: no `nvs` binary. Build one (`cargo build --release -p nvs-cli`) or pass --nvs.");
    return flush(1);
  }
  if (executes && bin === null) bin = namedBinary(nvs);
  if (blessed !== null) {
    const done = await bless(nvs, blessed.map(repoPath));
    out.push(...done.lines);
    return flush(done.failed ? 1 : 0);
  }
  const { policy, skips } = loadPolicy(flags.has("--no-perf"));
  const columns = shownProofs(policy);
  let entries: Entry[];
  try {
    entries = await roster(nvs);
  } catch (e) {
    if (!(e instanceof RosterError)) throw e;
    console.error(`nv proofs: ${e.message}`);
    return 1;
  }
  const group = values.get("--group");
  // The audit takes the last `--group`, as argparse does. `--run`, `--verify` and `--record-perf` take each one.
  const groups = executes || measures ? groupsIn(args) : group === undefined ? [] : [group];
  let scope = entries;
  if (groups.length > 0) {
    const missing = groups.find((g) => !entries.some((e) => e.group === g));
    if (missing !== undefined) {
      out.push(`nv proofs: no group ${pyRepr(missing)}.`);
      return flush(1);
    }
    scope = entries.filter((e) => groups.includes(e.group));
  }
  if (only !== null) {
    const wanted = new Set(only);
    scope = scope.filter((e) => wanted.has(e.id));
    // A named feature no longer on the roster was renamed or removed. A scope that silently narrowed to
    // nothing would pass.
    const onRoster = new Set(scope.map((e) => e.id));
    const unknown = [...wanted].filter((id) => !onRoster.has(id)).sort();
    if (unknown.length) {
      out.push(`nv proofs: --only names ${unknown.length} feature(s) that are not on the roster: ${unknown.slice(0, 5).join(", ")}`);
      return flush(1);
    }
  }
  const label = group ?? (only !== null ? `${only.length} named feature(s)` : null);
  const proofs = collect(entries);

  if (measures) {
    const fid = values.get("--id");
    const match = fid === undefined ? undefined : entries.find((e) => e.id === fid);
    if (fid !== undefined && !match) {
      out.push(`nv proofs: no feature ${pyRepr(fid)}.`);
      return flush(1);
    }
    const opts = { reps, note: values.get("--note") ?? "", force: flags.has("--force") };
    const rc = await recordPerf(out, nvs, match ? [match] : scope, proofs, policy, skips, opts);
    return flush(rc || (flags.has("--perf-report") ? perfReport(out) : 0));
  }

  if (executes) {
    const fid = values.get("--id");
    let scopes: Scope[];
    if (fid !== undefined) {
      const match = entries.find((e) => e.id === fid);
      if (!match) {
        out.push(`nv proofs: no feature ${pyRepr(fid)}.`);
        return flush(1);
      }
      scopes = [{ label: fid, entries: [match] }];
    } else if (groups.length > 1) {
      scopes = groups.map((g) => ({ label: g, entries: scope.filter((e) => e.group === g) }));
    } else {
      scopes = [{ label, entries: scope }];
    }
    // A narrowed group reads less than the whole of it, so only a whole group's paths are recorded.
    if (fid === undefined && only === null) {
      saveReads(scopes.filter((s) => s.label !== null && groups.includes(s.label)).map((s) => [s.label!, groupReads(s.entries)]));
    }
    return flush(await runScopes(out, bin!, scopes, flags.has("--verify"), flags, proofs, policy, skips));
  }

  if (flags.has("--gate")) return flush(gate(out, scope, proofs, policy, skips, label));
  if (flags.has("--json")) {
    const doc = scope.map((e) => {
      const p = proofs.get(e.id)!;
      return {
        id: e.id,
        kind: e.kind,
        group: e.group,
        path: e.path,
        anchor: e.anchor,
        owed: owed(e, p, policy, skips),
        have: { tests: p.nvst.length + p.rust.length, examples: p.examples.length, hostile: p.hostile.length, perf: Boolean(p.perf) },
      };
    });
    out.push(pythonJson(doc));
    return flush(0);
  }
  if (flags.has("--gaps")) {
    const rows = scope.flatMap((e) => proofs.get(e.id)!.gaps.map((f) => [e, f] as const));
    out.push(`== BUGS THE PROOFS FOUND  (${rows.length} marked proof(s))`);
    out.push("-- each is a real failure a session could not fix in the slice that found it, and is");
    out.push("-- recorded as a gap under `data/gaps/`. Removing the marker is part of the");
    out.push("-- fix: a marked proof that passes fails the sweep.");
    out.push("");
    for (const [e, f] of rows) {
      out.push(`  ${ljust(e.id, 44)} ${f}`);
      out.push(`  ${ljust("", 44)}   -> ${gapLine(f)}`);
    }
    return flush(0);
  }
  const fid = values.get("--id");
  if (fid !== undefined) return flush(printEntry(out, fid, entries, proofs, policy, skips));
  if (flags.has("--owed")) {
    printOwed(out, scope, proofs, policy, skips, limit || Number.MAX_SAFE_INTEGER);
    return flush(0);
  }
  if (group !== undefined) {
    printGroup(out, group, entries, proofs, policy, skips, columns);
    return flush(0);
  }
  printStatus(out, groupRows(entries, proofs, policy, skips), columns);
  return flush(0);
}
