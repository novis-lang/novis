// The acceptance sweep's order and verdicts: the half of the driver's sweep that decides, from a check
// record and what its run printed, whether it is green. The judging functions are exported on their
// own, because each is a pure function of a check and what its process printed, and
// `tools/nv/test/accept.test.ts` pins them from recorded output. What runs a check is
// `driver/runner.ts`, on the selection store.
//
// `acceptance` is the whole sweep: `tiers` puts every check in the order the sweep runs it, a check the
// change does not reach is not started, and the sweep stops at its first red unless it collects. The
// tiers, in order: stage 0's catch-up, the `setup` commands, the floor's fixtures, the cargo and command
// checks by stage, the goal's own fixtures, the `overlap` commands and the checks that build or measure
// the release profile. A fixture tier runs to its end and reports its reds as one line, earliest stage
// first. An `overlap` command starts when the setup tier ends and is judged after the goal's fixtures.
// The release tier's builds start when the overlap tier does, beside those commands, and so does what
// the caller hands in as `beside`, the Linux legs at a goal end. The release checks themselves run one
// at a time once every build, overlap command and `beside` run has ended, so a perf guard measures on
// cores nothing else of the sweep is using.
//
// A check is reached when the selection picks one of its atoms (`select/checks.ts`): something it was
// seen to use changed, it is new, red or owed, or its record changed. A check with `memoize = false`
// reads something outside the tree, so every sweep that holds it reaches it. A check the change does
// not reach prints nothing of its own, and its verdict is what the store holds: its atoms were green,
// and what the plan asks beyond them, a test a check names or a suite's `minPassing`, is judged from
// the store's last green runs. A suite below its `minPassing` is `short`, reported and never green.
//
// The floor gate is the turn's (`cmd/loop.ts`), and the WSL leg and the valgrind sweep are
// `driver/legs.ts`'s, which the turn starts as `beside`.

import { dirname } from "node:path";

/** A check as the goal record holds it. */
export interface Check {
  id: string;
  kind: string;
  stage: number;
  name?: string;
  argv?: string[];
  args?: string[];
  cwd?: string;
  tests?: string[];
  cases?: string[];
  file?: string;
  stream?: "stdout" | "stderr";
  want?: string[];
  exit?: "nonzero";
  stdoutContains?: string[];
  stderrContains?: string[];
  minBytes?: number;
  minPassing?: number;
  needs?: string;
  setup?: boolean;
  memoize?: boolean;
  overlap?: boolean;
}

/** What one process did: its exit status and both streams, kept apart. */
export interface Outcome {
  code: number;
  out: string;
  err: string;
}

/** A check's verdict: `fail` is the ledger's line, "" when green; `short` is a suite below its `minPassing`. */
export interface Verdict {
  fail: string;
  short: string;
}

export const PROGRAM_KINDS = new Set(["exact", "ordered", "contains", "min-bytes"]);

/** `nvs test`'s summary line. */
export const SUMMARY_RE = /(\d+)\s+passed,\s+(\d+)\s+failed/;

/** The line `bun run` adds when a package script fails, which says nothing the script did not. */
const SCRIPT_EXIT = /^error: script "[^"]*" exited with code -?\d+$/;

/** The lines of stdout's tail a verdict quotes when neither stream names the failure. */
const TAIL_LINES = 3;

/**
 * The one line that names a failure, which every verdict quotes. A `cargo test` run names the failed test
 * on stdout and says only `error: test failed` on stderr, after the build's warnings, so the pick is the
 * most specific line either stream holds: a failed test, then a panic (Rust's or Bun's own), then a
 * failed proof program, then the first `error` line, then stderr's first line, then stdout's last few
 * joined by ` | `. `bun run`'s own `error: script ... exited with code N` is the pick only when the script
 * printed nothing else: a red `bun nv` check quotes what the script printed.
 */
export function firstErrLine(r: Outcome): string {
  const lines = (text: string) => (text.trim() === "" ? [] : text.trim().split(/\r?\n/).filter((l) => !SCRIPT_EXIT.test(l.trim())));
  const err = lines(r.err);
  const out = lines(r.out);
  const both = [...out, ...err];
  const picks: ((l: string) => boolean)[] = [
    (l) => l.startsWith("test ") && l.endsWith("... FAILED"),
    (l) => l.startsWith("thread '") && l.includes("panicked at"),
    (l) => l.startsWith("panic("),
    (l) => /^\s*FAIL\s/.test(l),
    (l) => l.startsWith("error"),
  ];
  for (const pick of picks) {
    const hit = both.find(pick);
    if (hit !== undefined) return hit.trim();
  }
  if (err.length > 0) return err[0]!;
  const tail = out
    .filter((l) => l.trim() !== "")
    .slice(-TAIL_LINES)
    .map((l) => l.trim())
    .join(" | ");
  // A script that printed nothing else has only `bun run`'s line to quote.
  return tail || r.err.trim().split(/\r?\n/)[0]!;
}

/** `text`'s lines with CRLF normalised away and the trailing newline dropped. */
export function stdoutLines(text: string): string[] {
  return text.replaceAll("\r", "").replace(/\n+$/, "").split("\n");
}

/** Whether each of `wanted` appears in `text`, each one after the end of the one before it. */
export function orderedIn(text: string, wanted: string[]): boolean {
  let at = 0;
  for (const w of wanted) {
    const i = text.indexOf(w, at);
    if (i < 0) return false;
    at = i + w.length;
  }
  return true;
}

/** `[crate, target]` for `cargo test -p <crate>`, bare or with one `--test <name>`; null for anything else. */
export function plainCrateTest(args: string[]): [string, string | null] | null {
  if (args.length >= 3 && args[0] === "test" && args[1] === "-p") {
    if (args.length === 3) return [args[2]!, null];
    if (args.length === 5 && args[3] === "--test") return [args[2]!, args[4]!];
  }
  return null;
}

/** libtest flags after `--` that change how a run reports and never which tests it runs. */
const REPORTING = /^(?:--exact|--nocapture|--quiet|-q|--test-threads(?:=\d+)?|\d+)$/;

/**
 * The executables `args` runs a subset of, when `args` is `test -p <crate>` with at most one target,
 * `--lib`, `--bin <n>` or `--test <n>`, then name filters, then reporting flags after `--`. A whole run
 * of those executables that passed answers such a check. Null for any other flag, which can change
 * what runs: `--ignored`, `--features`, `--release`.
 */
export function subsetRun(args: string[]): { crate: string; kind: string | null; target: string | null } | null {
  if (args[0] !== "test" || args[1] !== "-p" || !args[2]) return null;
  let i = 3;
  let kind: string | null = null;
  let target: string | null = null;
  if (args[i] === "--lib") {
    kind = "lib";
    i++;
  } else if ((args[i] === "--bin" || args[i] === "--test") && args[i + 1]) {
    kind = args[i]!.slice(2);
    target = args[i + 1]!;
    i += 2;
  }
  for (; i < args.length && args[i] !== "--"; i++) if (args[i]!.startsWith("-")) return null;
  for (i++; i < args.length; i++) if (!REPORTING.test(args[i]!)) return null;
  return { crate: args[2]!, kind, target };
}

/** Whether `c` reads `target/release/nvs`, which only a bench measures and nothing else here builds. */
export function measuresReleaseCli(c: Check): boolean {
  const argv = c.argv ?? [];
  return c.kind === "command" && argv[0] === "bun" && argv[1] === "nv" && argv[2] === "bench";
}

/** A fixture's verdict from its run, "" when green. */
export function judgeProgram(c: Check, r: Outcome, label: string): string {
  const wantsNonzero = c.exit === "nonzero";
  if (wantsNonzero && r.code === 0) return `${label}: exited 0, wanted non-zero`;
  if (!wantsNonzero && r.code !== 0) return `${label}: exit ${r.code} -- ${firstErrLine(r)}`;
  const want = c.want ?? [];
  const text = c.stream === "stderr" ? r.err : r.out;
  switch (c.kind) {
    case "exact": {
      const got = stdoutLines(r.out);
      if (got.length !== want.length || got.some((l, i) => l !== want[i])) {
        return `${label}: stdout was [${got.join(" / ")}], wanted [${want.join(" / ")}]`;
      }
      return "";
    }
    case "ordered":
      return orderedIn(text, want) ? "" : `${label}: ${c.stream ?? "stdout"} lacks, in order: ${want.join(" -> ")}`;
    case "contains":
      for (const needle of c.stderrContains ?? []) if (!r.err.includes(needle)) return `${label}: stderr lacks ${JSON.stringify(needle)}`;
      for (const needle of c.stdoutContains ?? []) if (!r.out.includes(needle)) return `${label}: stdout lacks ${JSON.stringify(needle)}`;
      return "";
    case "min-bytes":
      return text.length < (c.minBytes ?? 0) ? `${label}: only ${text.length} bytes of output, wanted ${c.minBytes}` : "";
    default:
      return `${label}: unknown check kind ${JSON.stringify(c.kind)}`;
  }
}

/** A `command` check's verdict: its exit, then `want` in order across both streams. */
export function judgeCommand(c: Check, r: Outcome, label: string): string {
  const wantsNonzero = c.exit === "nonzero";
  if (wantsNonzero && r.code === 0) return `${label}: exit 0, and this check asserts a non-zero exit`;
  if (!wantsNonzero && r.code !== 0) return `${label}: exit ${r.code} -- ${firstErrLine(r)}`;
  const want = c.want ?? [];
  return orderedIn(`${r.out}\n${r.err}`, want) ? "" : `${label}: output lacks, in order: ${want.join(" -> ")}`;
}

/**
 * An `nvs-suite` or `cargo-named` check's verdict. A suite needs its summary line with no failure, each
 * named case on disk and not skipped, and `minPassing` cases passing. That last one is the stopping
 * condition rather than a correctness signal, so it comes back as `short` and the rest can still pass.
 */
export function judgeTests(c: Check, r: Outcome, label: string, onDisk: (rel: string) => boolean): Verdict {
  if (r.code !== 0) return { fail: `${label}: exit ${r.code} -- ${firstErrLine(r)}`, short: "" };
  const both = `${r.out}\n${r.err}`;
  if (c.kind === "cargo-named") {
    const missing = (c.tests ?? []).find((name) => !both.includes(name));
    return { fail: missing === undefined ? "" : `${label}: test ${JSON.stringify(missing)} did not run`, short: "" };
  }
  if (c.kind !== "nvs-suite") return { fail: `${label}: unknown check kind ${JSON.stringify(c.kind)}`, short: "" };
  const m = SUMMARY_RE.exec(both);
  if (m === null) return { fail: `${label}: no 'N passed, M failed' summary line in the output`, short: "" };
  if (Number(m[2]) !== 0) return { fail: `${label}: ${m[2]} case(s) failed`, short: "" };
  // `nvs test` prints the platform's separator, so a case is compared with forward slashes on both sides.
  const flat = both.replaceAll("\\", "/");
  for (const kase of c.cases ?? []) {
    const want = kase.replaceAll("\\", "/");
    if (!onDisk(kase)) return { fail: `${label}: case ${want} is not written yet`, short: "" };
    if (flat.includes(`SKIP ${want}`)) return { fail: `${label}: case ${want} was skipped, so nothing ran it`, short: "" };
  }
  const floor = c.minPassing;
  const short = floor !== undefined && Number(m[1]) < floor ? `${label}: only ${m[1]} passing case(s), wanted at least ${floor}` : "";
  return { fail: "", short };
}

/** One test executable of the workspace build: its target name, its path and its package's directory. */
export interface TestExe {
  target: string;
  /** `lib`, `bin` or `test`. */
  kind: string;
  exe: string;
  dir: string;
}

/**
 * Every test executable in `cargo test --no-run --message-format=json`'s output, by package name. The
 * package id is `path+file:///…/crates/nvs-types#0.0.1`, or `…/benches/abi-probe#nvs-abi-probe@0.0.1`
 * when the directory and the package differ.
 */
export function testExecutables(json: string): Map<string, TestExe[]> {
  const exes = new Map<string, TestExe[]>();
  for (const line of json.split(/\r?\n/)) {
    let m: any;
    try {
      m = JSON.parse(line);
    } catch {
      continue;
    }
    if (m?.reason !== "compiler-artifact" || !m.executable || !m.profile?.test) continue;
    const kinds: string[] = m.target?.kind ?? [];
    const kind = kinds.find((k) => k === "lib" || k === "bin" || k === "test");
    if (kind === undefined) continue;
    const pid: string = m.package_id ?? "";
    const hash = pid.lastIndexOf("#");
    const source = hash < 0 ? "" : pid.slice(0, hash);
    const tail = hash < 0 ? pid : pid.slice(hash + 1);
    const name = tail.includes("@") ? tail.split("@", 1)[0]! : source.replace(/\/+$/, "").split("/").pop()!;
    const list = exes.get(name) ?? [];
    list.push({ target: m.target.name, kind, exe: m.executable, dir: dirname(m.manifest_path) });
    exes.set(name, list);
  }
  return exes;
}

/** The groups a `bun nv proofs --verify --group G...` check names, in order; null for any other check. */
export function proofGroups(c: Check): string[] | null {
  const argv = c.argv ?? [];
  if (c.kind !== "command" || (c.cwd ?? ".") !== "." || argv.slice(0, 4).join(" ") !== "bun nv proofs --verify") return null;
  const groups: string[] = [];
  for (let i = 4; i < argv.length; i++) {
    if (argv[i] === "--group" && i + 1 < argv.length) groups.push(argv[++i]!);
    else if (argv[i]!.startsWith("--group=")) groups.push(argv[i]!.slice("--group=".length));
    else return null;
  }
  return groups.length > 0 ? groups : null;
}

/** The features a `bun nv proofs --verify --only ID...` check names, in order; null for any other check. */
export function proofOnly(c: Check): string[] | null {
  const argv = c.argv ?? [];
  if (c.kind !== "command" || (c.cwd ?? ".") !== "." || argv.slice(0, 5).join(" ") !== "bun nv proofs --verify --only") return null;
  const ids = argv.slice(5);
  return ids.length > 0 && ids.every((a) => !a.startsWith("-")) ? ids : null;
}

/**
 * One `nv proofs --verify` process that answers several checks: every group their `--group` forms
 * name, and one `--only-as` scope per distinct feature list their `--only` forms name. `parts` gives
 * each check's scopes by check id, which are the sections it is judged on.
 */
export interface ProofBatch {
  groups: string[];
  named: { label: string; ids: string[] }[];
  parts: Map<string, string[]>;
}

/**
 * The one batch the `nv proofs --verify` checks among `checks` run in, in check order: each group once,
 * and each distinct `--only` list once under a label no group of the batch has. A batch of fewer than
 * two scopes is none, and null: its check runs alone.
 */
export function proofBatch(checks: Check[]): ProofBatch | null {
  const groups: string[] = [];
  const named: { label: string; ids: string[] }[] = [];
  const lists = new Map<string, string>();
  const pending: { c: Check; ids: string[] }[] = [];
  const parts = new Map<string, string[]>();
  for (const c of checks) {
    const own = proofGroups(c);
    if (own !== null) {
      for (const g of own) if (!groups.includes(g)) groups.push(g);
      parts.set(c.id, [...new Set(own)]);
      continue;
    }
    const ids = proofOnly(c);
    if (ids !== null) pending.push({ c, ids });
  }
  let n = 0;
  for (const { c, ids } of pending) {
    const key = ids.join("\0");
    let label = lists.get(key);
    if (label === undefined) {
      do label = `only-${++n}`;
      while (groups.includes(label));
      lists.set(key, label);
      named.push({ label, ids });
    }
    parts.set(c.id, [label]);
  }
  return groups.length + named.length > 1 ? { groups, named, parts } : null;
}

/** The command line of `batch`'s one process. */
export function proofBatchArgv(batch: ProofBatch): string[] {
  return ["bun", "nv", "proofs", "--verify", ...batch.groups.flatMap((g) => ["--group", g]), ...batch.named.flatMap((n) => ["--only-as", n.label, ...n.ids])];
}

/** How many scopes of a batch one check's time limit covers. A whole sweep's proofs, every program of
 * every scope reached, take about that limit per this many scopes. */
export const PROOF_SCOPES_PER_LIMIT = 12;

/**
 * The time limit of a batch of `scopes` scopes: `base`, one check's limit, for each started
 * `PROOF_SCOPES_PER_LIMIT` of them. A hung program is stopped by its own limit inside the process
 * (`proofs/run.ts`), so this one ends only a process that stops making progress as a whole.
 */
export function proofLimitMs(scopes: number, base: number): number {
  return base * Math.max(1, Math.ceil(scopes / PROOF_SCOPES_PER_LIMIT));
}

/** Whether a folded run's stdout closes a section, with its verdict, for each of `parts`. A check it does
 * not is run again on its own. */
export function proofComplete(parts: string[], out: string): boolean {
  const sections = proofSections(out);
  return parts.every((p) => sections.has(p));
}

/**
 * Each scope's section of a batched `nv proofs --verify` run's stdout, from its `== G` line to its
 * `-- G: passed` or `-- G: failed` line, with both kept, and whether it passed. `G` is a group, or the
 * label of an `--only-as` scope.
 */
export function proofSections(out: string): Map<string, { passed: boolean; lines: string[] }> {
  const sections = new Map<string, { passed: boolean; lines: string[] }>();
  let group: string | null = null;
  let lines: string[] = [];
  for (const line of stdoutLines(out)) {
    if (line.startsWith("== ")) {
      group = line.slice(3);
      lines = [line];
    } else if (group !== null) {
      lines.push(line);
      if (line === `-- ${group}: passed` || line === `-- ${group}: failed`) {
        sections.set(group, { passed: line.endsWith(": passed"), lines });
        group = null;
      }
    }
  }
  return sections;
}

/**
 * What `nv proofs --verify` over `groups` alone would have done, cut from a batched run over more scopes.
 * One scope prints its lines with no `==` and `--` lines around them, as a run over one group, or one
 * `--only` list, does. A scope the batch printed no verdict for fails with the batch's own output, which
 * says why it stopped, and a last line naming the scope.
 */
export function proofOutcome(groups: string[], batch: Outcome): Outcome {
  const sections = proofSections(batch.out);
  const missing = groups.find((g) => !sections.has(g));
  if (missing !== undefined) {
    const why = [batch.err.trim(), `the batched \`nv proofs --verify\` printed no verdict for ${missing}`].filter((l) => l !== "");
    return { code: batch.code === 0 ? 1 : batch.code, out: batch.out, err: why.join("\n") };
  }
  const mine = groups.map((g) => sections.get(g)!);
  const lines = groups.length === 1 ? mine[0]!.lines.slice(1, -1) : mine.flatMap((s) => s.lines);
  return { code: mine.every((s) => s.passed) ? 0 : 1, out: `${lines.join("\n")}\n`, err: "" };
}

/**
 * The footprint of a check over the scopes `groups`, cut from a batched run: what the run read outside
 * every scope together with what each of `groups` read, which `parts` holds by scope, and not what the
 * batch's other scopes read. A scope with no part of its own takes the whole batch's `keys`, which is
 * never narrower than what it read.
 */
export function proofKeys(groups: string[], batch: { keys: Map<string, string>; parts: Map<string, Map<string, string>> }): Map<string, string> {
  const own = groups.map((g) => batch.parts.get(g));
  if (own.some((k) => k === undefined)) return new Map(batch.keys);
  return new Map(own.flatMap((k) => [...k!]));
}

/** What a recorded run printed, what it read, and what each of its parts read. */
export interface ProofRun {
  o: Outcome;
  keys: Map<string, string>;
  parts: Map<string, Map<string, string>>;
}

/**
 * A proofs check's outcome and footprint. With `batch`, the check's scopes `parts` are cut from the
 * batch's one run. When that run printed no section for one of them, because it died, was killed or
 * stopped before its verdicts, the check runs `alone`, as it does outside a batch, so a crash never turns
 * red a check that passes on its own. `alone` gets the words its progress line ends on: why it runs
 * alone, or nothing outside a batch.
 */
export async function proofResult(parts: string[], batch: (() => Promise<ProofRun>) | null, alone: (why: string) => Promise<ProofRun>): Promise<{ o: Outcome; keys: Map<string, string> }> {
  let why = "";
  if (batch !== null) {
    const got = await batch();
    if (proofComplete(parts, got.o.out)) return { o: proofOutcome(parts, got.o), keys: proofKeys(parts, got) };
    why = ` (run alone: the batched run ${got.o.code === -1 ? "did not finish" : `ended with exit ${got.o.code}`} and printed no verdict for it)`;
  }
  const got = await alone(why);
  return { o: got.o, keys: new Map(got.keys) };
}

// ---- the whole sweep -------------------------------------------------------------------------------

/** One tier of the sweep: its checks in run order, and whether they are fixtures, reported as one line. */
export interface Tier {
  name: string;
  checks: Check[];
  programs: boolean;
}

/** A check carried in as the floor: stage 0's catch-up, or a stage whose label says `floor`. */
export function isFloor(c: Check, label: string): boolean {
  return c.stage === 0 || label.toLowerCase().includes("floor");
}

/** Whether `c` builds or measures the release profile, which the sweep runs last whatever its stage. */
export function isRelease(c: Check): boolean {
  return (c.args ?? []).includes("--release") || measuresReleaseCli(c);
}

/**
 * The cargo command that builds what release check `c` runs and runs nothing: its own arguments before
 * any `--`, with `--no-run` for a test or bench. `null` for a check that is not a release cargo run.
 */
export function releaseBuildArgv(c: Check): string[] | null {
  const args = c.args ?? [];
  if (c.kind !== "cargo-named" || !args.includes("--release")) return null;
  const sep = args.indexOf("--");
  const head = sep < 0 ? args : args.slice(0, sep);
  if (head[0] === "test" || head[0] === "bench") return ["cargo", ...head, "--no-run"];
  if (head[0] === "build") return ["cargo", ...head];
  return null;
}

/** Does this command run what `needle` names? A `git grep` that only quotes it does not. */
function runs(argv: string[], needle: string): boolean {
  return argv.length > 0 && argv[0] !== "git" && argv.some((a) => a.includes(needle));
}

/** The heavy runs a command check can be, besides the release profile: a fuzz target, ThreadSanitizer,
 * or the database matrix. Each builds what no recorded run builds. */
export function heavyRole(c: Check): "fuzz" | "tsan" | "db-matrix" | null {
  if (c.kind !== "command") return null;
  const argv = c.argv ?? [];
  if (runs(argv, "cargo +nightly fuzz run")) return "fuzz";
  if (runs(argv, "tools/tsan.sh")) return "tsan";
  if (argv[0] === "bun" && argv[1] === "nv" && argv[2] === "db-matrix") return "db-matrix";
  return null;
}

/**
 * Whether `c` is heavy and waits for the floor gate: it builds or measures the release profile, runs fuzz,
 * TSan or the database matrix, or is never memoized. Every other check runs in the sweep after the session
 * whose change reached it.
 */
export function isHeavy(c: Check): boolean {
  return c.memoize === false || isRelease(c) || heavyRole(c) !== null;
}

/**
 * Whether a shut floor gate holds `c` back: it is heavy and not a `setup` command. A setup is heavy only
 * because it is never memoized, and it writes what the fixtures read, such as the tables a migration
 * creates, so it runs before them whatever the gate.
 */
export function heldByGate(c: Check): boolean {
  return isHeavy(c) && c.setup !== true;
}

/**
 * `shown` with the plan's `setup` commands added when it holds a fixture, so a sweep narrowed by a filter
 * runs a fixture after its setup, as the whole plan does. `tiers` puts them in front.
 */
export function withSetups(shown: Check[], plan: Check[]): Check[] {
  if (!shown.some((c) => PROGRAM_KINDS.has(c.kind))) return shown;
  const ids = new Set(shown.map((c) => c.id));
  return [...shown, ...plan.filter((c) => c.setup === true && !ids.has(c.id))];
}

/** Every check in the tiers the sweep runs, each tier by stage and in the plan's order within one. */
export function tiers(checks: Check[], label: (n: number) => string): Tier[] {
  const byStage = (list: Check[]) => list.map((c, i) => ({ c, i })).sort((a, b) => a.c.stage - b.c.stage || a.i - b.i).map((x) => x.c);
  const programs = byStage(checks.filter((c) => PROGRAM_KINDS.has(c.kind)));
  const rest = checks.filter((c) => !PROGRAM_KINDS.has(c.kind));
  const plain = rest.filter((c) => !c.setup && !c.overlap);
  return [
    { name: "catch-up", checks: plain.filter((c) => c.stage === 0 && !isRelease(c)), programs: false },
    { name: "setup", checks: rest.filter((c) => c.setup), programs: false },
    { name: "floor fixtures", checks: programs.filter((c) => isFloor(c, label(c.stage))), programs: true },
    { name: "cargo and command checks", checks: byStage(plain.filter((c) => c.stage !== 0 && !isRelease(c))), programs: false },
    { name: "goal fixtures", checks: programs.filter((c) => !isFloor(c, label(c.stage))), programs: true },
    { name: "overlap", checks: rest.filter((c) => !c.setup && c.overlap), programs: false },
    { name: "release", checks: plain.filter(isRelease), programs: false },
  ];
}

/** A fixture tier's reds as one line: the earliest in full, then up to six more by name. */
export function programFailLine(fails: { c: Check; fail: string }[], label: (n: number) => string): string {
  const first = fails[0]!.fail;
  if (fails.length === 1) return first;
  const others: string[] = [];
  for (const { c } of fails.slice(1)) {
    const name = `${c.file ?? c.name ?? "?"} [${label(c.stage)}]`;
    if (!others.includes(name)) others.push(name);
  }
  const more = others.length > 6 ? `, +${others.length - 6} more` : "";
  return `${first}\n       (and ${fails.length - 1} later fixture(s) red, in stage order: ${others.slice(0, 6).join(", ")}${more})`;
}

/** A collecting sweep's verdict: its first red whole, then each other one's first line as `also red:`. */
export function allReds(reds: string[]): string {
  return [reds[0]!, ...reds.slice(1).map((r) => `       also red: ${r.split("\n")[0]!.trim()}`)].join("\n");
}

/** A build that fails stops every sweep, collecting or not, since nothing behind it can run. */
export const BUILD_FAILED = /^the (native|workspace test|release) build failed/;

/**
 * A check the goal switch carried in from the goals before: a stage whose label says `floor`. Stage 0 is
 * a floor to `isFloor` but is this goal's own reopened work, so it is not carried.
 */
export function isCarried(label: string): boolean {
  return label.toLowerCase().includes("floor");
}

/**
 * The carried checks the change reaches, over the tree as it stands: what `nv loop --owed` names and
 * `--settle` runs. The goal's own checks are left out, since they stay red until the goal is reached,
 * and so is a check with `memoize = false`, which every sweep reaches and every open floor gate runs.
 */
export function owedChecks(checks: Check[], label: (n: number) => string, reached: (c: Check) => boolean): Check[] {
  return checks.filter((c) => isCarried(label(c.stage)) && c.memoize !== false && reached(c));
}

export interface AcceptanceOptions {
  label: (n: number) => string;
  /** What runs and judges one check: `driver/runner.ts`'s `Runner`, or a stand-in under test. `batch` is
   * told each tier's reached checks before the first of them runs. `prebuild` is handed the release
   * tier's reached checks when the overlap tier starts, and builds what they will build; its promise
   * never rejects. */
  sweep: { check(c: Check): Promise<Verdict>; batch?(checks: Check[]): void; prebuild?(checks: Check[]): Promise<void> };
  /** Work that starts when the overlap tier does, beside the release builds, and that no measurement may
   * share the cores with: the release tier starts only once it ends, and the sweep never returns while it
   * runs. A sweep that stops at a red check before the overlap tier never starts it. Its promise never
   * rejects. */
  beside?: () => Promise<void>;
  /** Whether the change reaches `c`. A check it does not reach is not started and prints nothing. */
  reached: (c: Check) => boolean;
  /** The verdict of a check the change does not reach, from what the store holds; green when omitted. */
  judged?: (c: Check) => Verdict;
  /** Run past a red check and report every red one. */
  collect: boolean;
  /** Called with each reached check before it runs. */
  trace?: (c: Check) => void;
}

/** What a sweep found: `fail` is "" only when every check it judged is green and none is short. */
export interface AcceptanceResult {
  fail: string;
  /** The checks it ran. */
  ran: number;
  /** The checks the change did not reach, judged from the store. */
  answered: number;
  /** Each check it judged, by id: green, or not. */
  verdicts: Map<string, boolean>;
}

/** Runs `checks` as one sweep, in `tiers`' order. */
export async function acceptance(checks: Check[], o: AcceptanceOptions): Promise<AcceptanceResult> {
  const reds: string[] = [];
  const shorts: string[] = [];
  const verdicts = new Map<string, boolean>();
  let ran = 0;
  let answered = 0;
  const pending = new Map<string, Promise<Verdict>>();
  // The release builds and the caller's `beside` run, started beside the overlap commands. The sweep
  // never returns while one still runs, so nothing it measures after them, and no other tree's sweep,
  // shares the cores with them.
  let prebuilt: Promise<void> = Promise.resolve();
  let besides: Promise<void> = Promise.resolve();
  const settled = async () => {
    await prebuilt;
    await besides;
  };
  const done = async (fail: string): Promise<AcceptanceResult> => {
    await settled();
    return { fail, ran, answered, verdicts };
  };

  const order = tiers(checks, o.label);
  for (const tier of order) {
    const fails: { c: Check; fail: string }[] = [];
    o.sweep.batch?.(tier.checks.filter(o.reached));
    // The overlap tier waits on commands that started with the setup tier. The release tier's builds
    // and `beside` start now, beside them, and its measurements start only once all of those have ended.
    if (tier.name === "overlap") {
      const release = order.find((t) => t.name === "release")!.checks.filter(o.reached);
      if (o.sweep.prebuild && release.length > 0) prebuilt = o.sweep.prebuild(release);
      if (o.beside) besides = o.beside();
    }
    if (tier.name === "release") await settled();
    for (const c of tier.checks) {
      let v: Verdict;
      if (!o.reached(c)) {
        answered++;
        v = o.judged?.(c) ?? { fail: "", short: "" };
      } else {
        if (!pending.has(c.id)) o.trace?.(c);
        v = await (pending.get(c.id) ?? o.sweep.check(c));
        ran++;
      }
      verdicts.set(c.id, v.fail === "" && v.short === "");
      if (v.fail === "") {
        if (v.short !== "") shorts.push(v.short);
        continue;
      }
      if (BUILD_FAILED.test(v.fail)) return done(reds.length === 0 ? v.fail : allReds([...reds, v.fail]));
      if (tier.programs) fails.push({ c, fail: v.fail });
      else if (o.collect) reds.push(v.fail);
      else return done(v.fail);
    }
    if (fails.length > 0) {
      const line = programFailLine(fails, o.label);
      if (!o.collect) return done(line);
      reds.push(line);
    }
    // The overlap commands run beside every tier from here to their own.
    if (tier.name === "setup") {
      for (const c of checks.filter((c) => !PROGRAM_KINDS.has(c.kind) && !c.setup && c.overlap)) {
        if (!o.reached(c)) continue;
        o.trace?.(c);
        pending.set(c.id, o.sweep.check(c));
      }
    }
  }
  if (reds.length > 0) return done(allReds(reds));
  return done(shorts[0] ?? "");
}
