// One acceptance check run and judged: the half of the driver's sweep that turns a check record into a
// verdict. `Sweep.check` runs a check of any kind and returns the line a ledger quotes for it, or "" when
// it is green. The judging functions are exported on their own, because each is a pure function of a
// check and what its process printed, and `tools/nv/test/accept.test.ts` pins them from recorded output.
//
// A sweep shares every process between the checks that ask for the same one. Two checks naming one
// `argv` in one directory get one run, and each judges its own exit and `want` against it. The same holds
// for a cargo argument list, an `nvs test` argument list and a crate's test executables. The tree
// cannot change inside one sweep, so a second run would only answer the same thing again. The
// `nv proofs --verify --group` checks of one tier share a run too: the first one reached starts one
// `nv proofs --verify` over every group the tier's checks name, and each check is judged on its own
// groups' sections of what that run prints, so the roster is read and the pool filled once.
//
// A fixture and a suite run the debug CLI, built once per sweep by `cargo build` over the whole
// workspace. A `cargo test -p <crate>` check, bare or with one `--test <name>`, runs that crate's test
// executables off one `cargo test --no-run` of the whole workspace, where cargo would run them: in the
// package's own directory with `CARGO_MANIFEST_DIR` set. A `-p` debug build resolves features over one
// package and writes a second copy of every workspace crate, which AGENTS.md rule 5 forbids. A check that
// measures the release CLI gets `cargo build --release -p nvs-cli` first, since nothing else builds it.
//
// `acceptance` is the whole sweep: `tiers` puts every check in the order the sweep runs it, a check the
// memo answers is not run, and the sweep stops at its first red unless it collects. The tiers, in order:
// stage 0's catch-up, the `setup` commands, the floor's fixtures, the cargo and command checks by stage,
// the goal's own fixtures, the `overlap` commands and the checks that build or measure the release
// profile. A fixture tier runs to its end and reports its reds as one line, earliest stage first. An
// `overlap` command starts when the setup tier ends and is judged after the goal's fixtures.
//
// A `bun nv` check runs with `NV_READS_LOG` set, and what its processes read is kept in
// `.loop/nv-reads.json` for its next key (`lib/reads.ts`, `keys/checks.ts`).
//
// `GreenMemo` remembers a green check under its id and the key `nv why` prints for it, which is every
// input the check reads. The memo answers a check green only while that key is unchanged. A check with
// `memoize = false` reads something outside the tree, so the memo never answers it. A suite below its
// `minPassing` is not remembered, since the next sweep must report it again.
//
// The floor gate is the turn's (`cmd/loop.ts`), and the WSL leg and the valgrind sweep are
// `driver/legs.ts`'s, run after these tiers. A sweep does not reuse `nv verify`'s green test records: a
// test a check names runs here whatever `verify` last found.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { recordId, roleOf, saveNvReads } from "../keys/checks.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { ENV as READS_ENV, readLog } from "../lib/reads.ts";
import { linked, releaseCli } from "../lib/relink.ts";

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

const DEFAULT_TIMEOUT_MS = 1800 * 1000;

/**
 * The one line that names a failure, which every verdict quotes. A `cargo test` run names the failed test
 * on stdout and says only `error: test failed` on stderr, after the build's warnings, so the pick is the
 * most specific line either stream holds: a failed test, then a panic, then the first `error` line, then
 * stderr's first line, then stdout's last.
 */
export function firstErrLine(r: Outcome): string {
  const err = r.err.trim() === "" ? [] : r.err.trim().split(/\r?\n/);
  const out = r.out.trim() === "" ? [] : r.out.trim().split(/\r?\n/);
  const both = [...out, ...err];
  const picks: ((l: string) => boolean)[] = [
    (l) => l.startsWith("test ") && l.endsWith("... FAILED"),
    (l) => l.startsWith("thread '") && l.includes("panicked at"),
    (l) => l.startsWith("error"),
  ];
  for (const pick of picks) {
    const hit = both.find(pick);
    if (hit !== undefined) return hit;
  }
  return err[0] || (out[out.length - 1] ?? "");
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

/**
 * Each group's section of a batched `nv proofs --verify` run's stdout, from its `== G` line to its
 * `-- G: passed` or `-- G: failed` line, with both kept, and whether it passed.
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
 * What `nv proofs --verify` over `groups` alone would have done, cut from a batched run over more groups.
 * One group prints its lines with no `==` and `--` lines around them, as a run over one group does. A
 * group the batch printed no verdict for fails with the batch's own output, which says why it stopped.
 */
export function proofOutcome(groups: string[], batch: Outcome): Outcome {
  const sections = proofSections(batch.out);
  const missing = groups.find((g) => !sections.has(g));
  if (missing !== undefined) {
    return { code: batch.code === 0 ? 1 : batch.code, out: batch.out, err: batch.err.trim() !== "" ? batch.err : `the batched \`nv proofs --verify\` printed no verdict for ${missing}` };
  }
  const mine = groups.map((g) => sections.get(g)!);
  const lines = groups.length === 1 ? mine[0]!.lines.slice(1, -1) : mine.flatMap((s) => s.lines);
  return { code: mine.every((s) => s.passed) ? 0 : 1, out: `${lines.join("\n")}\n`, err: "" };
}

/** Runs `argv`, and turns a program that cannot start into exit -1 with the reason on stderr. */
async function capture(argv: string[], cwd: string = ROOT, env?: Record<string, string>, timeoutMs = DEFAULT_TIMEOUT_MS): Promise<Outcome> {
  try {
    const r = await run(argv, env === undefined ? { cwd, timeoutMs } : { cwd, env, timeoutMs });
    return { code: r.timedOut ? -1 : r.code, out: r.stdout, err: r.timedOut ? `timed out after ${timeoutMs / 1000}s` : r.stderr };
  } catch (e) {
    return { code: -1, out: "", err: String((e as Error).message ?? e) };
  }
}

let logs = 0;

/** Runs a `bun nv` check with what its processes read recorded (`lib/reads.ts`), and keeps the record
 * for the check's next key. A run that leaves no record keeps the one before. */
async function recorded(argv: string[], cwd: string): Promise<Outcome> {
  const log = join(ROOT, ".loop", "reads", `nv-${process.pid}-${++logs}.ndjson`);
  mkdirSync(dirname(log), { recursive: true });
  rmSync(log, { force: true });
  const r = await capture(argv, join(ROOT, cwd), { [READS_ENV]: log });
  const reads = readLog(log);
  rmSync(log, { force: true });
  if (reads !== null) saveNvReads(recordId(cwd, argv), reads);
  return r;
}

export interface SweepOptions {
  /** A check's stage as the ledger prints it, `9 the cutover`. */
  stageLabel: (n: number) => string;
  /** Called with each process before it starts, so a caller can say what the sweep is doing. */
  onRun?: (what: string) => void;
}

/** One sweep's processes, shared between the checks that ask for the same one. */
export class Sweep {
  private cli: Promise<string | { fail: string }> | undefined;
  private release: Promise<Outcome> | undefined;
  private exes: Promise<Map<string, TestExe[]> | { fail: string }> | undefined;
  private readonly memo = new Map<string, Promise<Outcome>>();
  /** Each test executable's whole run this sweep, by its path. */
  private readonly exeRuns = new Map<string, Promise<Outcome>>();
  private proofs: { groups: string[]; run?: Promise<Outcome> } | undefined;

  constructor(private readonly opts: SweepOptions) {}

  /**
   * Names the checks the sweep is about to reach, so the first `nv proofs --verify --group` check among
   * them runs every group they name in one process. Fewer than two groups is no batch, since a run over
   * one group prints no sections to cut. A batch already started is kept by the checks that await it.
   */
  batch(checks: Check[]): void {
    const groups = [...new Set(checks.flatMap((c) => proofGroups(c) ?? []))];
    this.proofs = groups.length > 1 ? { groups } : undefined;
  }

  private once(key: string, what: string, thunk: () => Promise<Outcome>): Promise<Outcome> {
    let p = this.memo.get(key);
    if (p === undefined) {
      this.opts.onRun?.(what);
      p = thunk();
      this.memo.set(key, p);
    }
    return p;
  }

  /** The debug CLI's path, built once by the workspace build `nv verify` also runs. */
  private binary(): Promise<string | { fail: string }> {
    this.cli ??= (async () => {
      this.opts.onRun?.("cargo build");
      const r = await capture(["cargo", "build", "--quiet"]);
      if (r.code !== 0) return { fail: `the native build failed -- ${firstErrLine(r)}` };
      return join(ROOT, "target", "debug", process.platform === "win32" ? "nvs.exe" : "nvs");
    })();
    return this.cli;
  }

  private testExes(): Promise<Map<string, TestExe[]> | { fail: string }> {
    this.exes ??= (async () => {
      this.opts.onRun?.("cargo test --no-run");
      const r = await capture(["cargo", "test", "--no-run", "--message-format=json"]);
      if (r.code !== 0) return { fail: `the workspace test build failed -- ${firstErrLine(r)}` };
      return testExecutables(r.out);
    })();
    return this.exes;
  }

  /** `cargo test -p <crate> [--test <target>]`'s outcome off the shared build, stopping at the first failure as cargo does. */
  private crateTests(crate: string, target: string | null): Promise<Outcome> {
    return this.once(`crate\0${crate}\0${target ?? ""}`, `${crate} tests`, async () => {
      const exes = await this.testExes();
      if (!(exes instanceof Map)) return { code: -1, out: "", err: exes.fail };
      const chosen = (exes.get(crate) ?? []).filter((t) => target === null || t.target === target);
      if (chosen.length === 0) {
        return { code: -1, out: "", err: `no test executable in the workspace build belongs to '${crate}'${target ? ` under the target '${target}'` : ""}` };
      }
      const outs: string[] = [];
      const errs: string[] = [];
      for (const t of chosen) {
        const one = await this.runExe(crate, t);
        outs.push(one.out);
        errs.push(one.err);
        if (one.code !== 0) {
          // libtest reports a failure on stdout, so the line the ledger reads from stderr names the tests.
          const failed = one.out.split(/\r?\n/).filter((l) => l.startsWith("test ") && l.trimEnd().endsWith("FAILED")).map((l) => l.split(/\s+/)[1]);
          errs.unshift(`${crate} (${t.target}): ${failed.length > 0 ? `${failed.length} test(s) failed: ${failed.join(", ")}` : `exit ${one.code} -- ${firstErrLine(one)}`}`);
          return { code: one.code, out: outs.join("\n"), err: errs.join("\n") };
        }
      }
      return { code: 0, out: outs.join("\n"), err: errs.join("\n") };
    });
  }

  /** One whole run of a test executable, run once per sweep whichever check reaches it first. */
  private runExe(crate: string, t: TestExe): Promise<Outcome> {
    let p = this.exeRuns.get(t.exe);
    if (p === undefined) {
      this.opts.onRun?.(`${crate}: ${t.target}`);
      p = capture([t.exe], t.dir, { CARGO_MANIFEST_DIR: t.dir });
      this.exeRuns.set(t.exe, p);
    }
    return p;
  }

  /** A `cargo test` run of a subset of executables this sweep has already run whole and seen pass,
   * answered from those runs; null when one of them has not run, or did not pass. */
  private async fromWholeRuns(args: string[]): Promise<Outcome | null> {
    const subset = subsetRun(args);
    if (subset === null) return null;
    const exes = await this.testExes();
    if (!(exes instanceof Map)) return null;
    const chosen = (exes.get(subset.crate) ?? []).filter(
      (t) => subset.kind === null || (t.kind === subset.kind && (subset.target === null || t.target === subset.target)),
    );
    if (chosen.length === 0 || chosen.some((t) => !this.exeRuns.has(t.exe))) return null;
    const runs = await Promise.all(chosen.map((t) => this.exeRuns.get(t.exe)!));
    if (runs.some((r) => r.code !== 0)) return null;
    return { code: 0, out: runs.map((r) => r.out).join("\n"), err: runs.map((r) => r.err).join("\n") };
  }

  /** `cargo test --workspace`'s outcome: every package's tests off the shared build, stopping at the first failure. */
  private async workspaceTests(): Promise<Outcome> {
    const exes = await this.testExes();
    if (!(exes instanceof Map)) return { code: -1, out: "", err: exes.fail };
    const outs: string[] = [];
    const errs: string[] = [];
    for (const crate of [...exes.keys()].sort()) {
      const one = await this.crateTests(crate, null);
      outs.push(one.out);
      errs.push(one.err);
      if (one.code !== 0) return { code: one.code, out: outs.join("\n"), err: errs.join("\n") };
    }
    return { code: 0, out: outs.join("\n"), err: errs.join("\n") };
  }

  private releaseCli(): Promise<Outcome> {
    this.release ??= (async () => {
      this.opts.onRun?.("cargo build --release -p nvs-cli");
      // An editor running `nvs lsp` out of this tree holds the binary the link replaces;
      // `tools/nv/lib/relink.ts` moves it aside so the retry lands.
      return linked(releaseCli(), () => capture(["cargo", "build", "--release", "-p", "nvs-cli"]), (r) => r.err);
    })();
    return this.release;
  }

  /** Runs `c` and judges it. */
  async check(c: Check): Promise<Verdict> {
    const label = `${c.name ?? c.file ?? c.id} [${this.opts.stageLabel(c.stage)}]`;
    const none = (fail: string): Verdict => ({ fail, short: "" });

    if (PROGRAM_KINDS.has(c.kind)) {
      // `needs = "af-unix"`: this platform's build has no Unix-domain transport, so the claim is not asked here.
      if (c.needs === "af-unix" && process.platform === "win32") return none("");
      const exe = await this.binary();
      if (typeof exe !== "string") return none(exe.fail);
      const args = [...(c.args ?? []), c.file ?? ""];
      const r = await this.once(`run\0${args.join("\0")}`, `nvs run ${args.join(" ")}`, () => capture([exe, "run", ...args]));
      return none(judgeProgram(c, r, `native ${c.file} [${this.opts.stageLabel(c.stage)}]`));
    }

    if (c.kind === "command") {
      const groups = proofGroups(c);
      const batch = this.proofs;
      if (groups !== null && batch !== undefined && groups.every((g) => batch.groups.includes(g))) {
        batch.run ??= (async () => {
          this.opts.onRun?.(`bun nv proofs --verify over ${batch.groups.length} groups`);
          return capture(["bun", "nv", "proofs", "--verify", ...batch.groups.flatMap((g) => ["--group", g])]);
        })();
        return none(judgeCommand(c, proofOutcome(groups, await batch.run), label));
      }
      let argv = c.argv ?? [];
      if (argv.includes("{nvs}")) {
        const exe = await this.binary();
        if (typeof exe !== "string") return none(exe.fail);
        argv = argv.map((a) => (a === "{nvs}" ? exe : a));
      }
      if (measuresReleaseCli(c)) {
        const built = await this.releaseCli();
        if (built.code !== 0) return none(`${label}: the release build failed -- ${firstErrLine(built)}`);
      }
      const cwd = c.cwd ?? ".";
      const nv = argv[0] === "bun" && argv[1] === "nv";
      const r = await this.once(`cmd\0${cwd}\0${argv.join("\0")}`, argv.join(" "), () => (nv ? recorded(argv, cwd) : capture(argv, join(ROOT, cwd))));
      return none(judgeCommand(c, r, label));
    }

    const args = c.args ?? [];
    let r: Outcome;
    if (c.kind === "nvs-suite") {
      const exe = await this.binary();
      if (typeof exe !== "string") return none(exe.fail);
      r = await this.once(`suite\0${args.join("\0")}`, `nvs ${args.join(" ")}`, () => capture([exe, ...args]));
    } else {
      const plain = plainCrateTest(args);
      if (plain !== null) r = await this.crateTests(plain[0], plain[1]);
      else if (args.length === 2 && args[0] === "test" && args[1] === "--workspace") r = await this.workspaceTests();
      else r = (await this.fromWholeRuns(args)) ?? (await this.once(`cargo\0${args.join("\0")}`, `cargo ${args.join(" ")}`, () => capture(["cargo", ...args])));
    }
    return judgeTests(c, r, label, (rel) => existsSync(join(ROOT, rel)));
  }
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

/** The roles of the checks that cost minutes: `keys/checks.ts`'s `roleOf`. */
const HEAVY_ROLES = new Set(["fuzz", "tsan", "db-matrix"]);

/**
 * Whether `c` is heavy and waits for the floor gate: it builds or measures the release profile, runs fuzz,
 * TSan or the database matrix, or is never memoized. Every other check runs in the sweep after the session
 * whose change reached it.
 */
export function isHeavy(c: Check): boolean {
  return c.memoize === false || isRelease(c) || HEAVY_ROLES.has(roleOf(c as unknown as Parameters<typeof roleOf>[0]));
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
const BUILD_FAILED = /^the (native|workspace test) build failed/;

/** The sweep's memo: each green check's id, and the key of what it read when it was green. */
export class GreenMemo {
  private dirty = false;

  constructor(private readonly green: Record<string, string> = {}) {}

  /** The memo at `path`, or an empty one when the file is absent or unreadable. */
  static load(path: string): GreenMemo {
    try {
      const got = JSON.parse(readFileSync(path, "utf8"))?.green;
      if (got && typeof got === "object" && !Array.isArray(got)) {
        return new GreenMemo(Object.fromEntries(Object.entries(got).filter((e): e is [string, string] => typeof e[1] === "string")));
      }
    } catch {
      // No memo yet: every check runs.
    }
    return new GreenMemo();
  }

  /** Whether `c` was green over exactly the inputs `key` names. */
  answers(c: Check, key: string | null): boolean {
    return c.memoize !== false && key !== null && this.green[c.id] === key;
  }

  remember(c: Check, key: string | null): void {
    if (c.memoize === false || key === null || this.green[c.id] === key) return;
    this.green[c.id] = key;
    this.dirty = true;
  }

  /** Writes the memo, keeping only the checks in `ids`: a check struck from the plan leaves nothing behind. */
  save(path: string, ids: Set<string>): void {
    const kept = Object.fromEntries(Object.entries(this.green).filter(([id]) => ids.has(id)));
    if (!this.dirty && Object.keys(kept).length === Object.keys(this.green).length) return;
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, `${JSON.stringify({ green: kept }, null, 1)}\n`);
    this.dirty = false;
  }
}

/**
 * A check the goal switch carried in from the goals before: a stage whose label says `floor`. Stage 0 is
 * a floor to `isFloor` but is this goal's own reopened work, so it is not carried.
 */
export function isCarried(label: string): boolean {
  return label.toLowerCase().includes("floor");
}

/**
 * The carried checks the memo does not answer for their keys: what `nv loop --owed` names and `--settle`
 * runs. The goal's own checks are left out, since they stay red until the goal is reached, and so is a
 * check with `memoize = false`, which no memo answers and every open floor gate runs.
 */
export function owedChecks(checks: Check[], label: (n: number) => string, memo: GreenMemo, key: (c: Check) => string | null): Check[] {
  return checks.filter((c) => isCarried(label(c.stage)) &&c.memoize !== false && !memo.answers(c, key(c)));
}

export interface AcceptanceOptions {
  label: (n: number) => string;
  /** What runs and judges one check: a `Sweep`, or a stand-in under test. `batch` is told each tier's checks the memo does not answer, before the first of them runs. */
  sweep: { check(c: Check): Promise<Verdict>; batch?(checks: Check[]): void };
  /** A check's key over the tree as the sweep began, or null when it has none. */
  key: (c: Check) => string | null;
  memo: GreenMemo;
  /** Consult no memo, and run every check. */
  full: boolean;
  /** Run past a red check and report every red one. */
  collect: boolean;
  /** Called with each check before it runs, and with each the memo answers. */
  trace?: (c: Check, answered: boolean) => void;
}

/** What a sweep found: `fail` is "" only when every check it reached is green and none is short. */
export interface AcceptanceResult {
  fail: string;
  ran: number;
  answered: number;
}

/** Runs `checks` as one sweep, in `tiers`' order. */
export async function acceptance(checks: Check[], o: AcceptanceOptions): Promise<AcceptanceResult> {
  const reds: string[] = [];
  const shorts: string[] = [];
  let ran = 0;
  let answered = 0;
  const pending = new Map<string, Promise<Verdict>>();
  const answer = (c: Check) => {
    if (o.full || !o.memo.answers(c, o.key(c))) return false;
    answered++;
    o.trace?.(c, true);
    return true;
  };
  const done = (fail: string): AcceptanceResult => ({ fail, ran, answered });

  for (const tier of tiers(checks, o.label)) {
    const fails: { c: Check; fail: string }[] = [];
    o.sweep.batch?.(tier.checks.filter((c) => o.full || !o.memo.answers(c, o.key(c))));
    for (const c of tier.checks) {
      if (answer(c)) continue;
      if (!pending.has(c.id)) o.trace?.(c, false);
      const v = await (pending.get(c.id) ?? o.sweep.check(c));
      ran++;
      if (v.fail === "") {
        if (v.short !== "") shorts.push(v.short);
        else o.memo.remember(c, o.key(c));
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
        if (o.full || !o.memo.answers(c, o.key(c))) {
          o.trace?.(c, false);
          pending.set(c.id, o.sweep.check(c));
        }
      }
    }
  }
  if (reds.length > 0) return done(allReds(reds));
  return done(shorts[0] ?? "");
}
