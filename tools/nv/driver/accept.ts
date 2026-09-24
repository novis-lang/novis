// One acceptance check run and judged: the half of the driver's sweep that turns a check record into a
// verdict. `Sweep.check` runs a check of any kind and returns the line a ledger quotes for it, or "" when
// it is green. The judging functions are exported on their own, because each is a pure function of a
// check and what its process printed, and `tools/nv/test/accept.test.ts` pins them from recorded output.
//
// A sweep shares every process between the checks that ask for the same one. Two checks naming one
// `argv` in one directory get one run, and each judges its own exit and `want` against it. The same holds
// for a cargo argument list, an `nvs test` argument list and a crate's test executables. The tree
// cannot change inside one sweep, so a second run would only answer the same thing again.
//
// A fixture and a suite run the debug CLI, built once per sweep by `cargo build` over the whole
// workspace. A `cargo test -p <crate>` check, bare or with one `--test <name>`, runs that crate's test
// executables off one `cargo test --no-run` of the whole workspace, where cargo would run them: in the
// package's own directory with `CARGO_MANIFEST_DIR` set. A `-p` debug build resolves features over one
// package and writes a second copy of every workspace crate, which AGENTS.md rule 5 forbids. A check that
// measures the release CLI gets `cargo build --release -p nvs-cli` first, since nothing else builds it.
//
// Not here yet, and each is the Python driver's until it is: the memo that answers a check green without
// running it, reusing `nv verify`'s green test records, one batched `nv proofs --verify` over every proofs
// check, the tiers and their order, the WSL leg and the valgrind sweep.

import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";

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

/** Whether `c` reads `target/release/nvs`, which only a bench measures and nothing else here builds. */
export function measuresReleaseCli(c: Check): boolean {
  const argv = c.argv ?? [];
  return c.kind === "command" && (argv.some((a) => a.includes("bench.py")) || (argv[0] === "bun" && argv[1] === "nv" && argv[2] === "bench"));
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
    if (!kinds.some((k) => k === "lib" || k === "bin" || k === "test")) continue;
    const pid: string = m.package_id ?? "";
    const hash = pid.lastIndexOf("#");
    const source = hash < 0 ? "" : pid.slice(0, hash);
    const tail = hash < 0 ? pid : pid.slice(hash + 1);
    const name = tail.includes("@") ? tail.split("@", 1)[0]! : source.replace(/\/+$/, "").split("/").pop()!;
    const list = exes.get(name) ?? [];
    list.push({ target: m.target.name, exe: m.executable, dir: dirname(m.manifest_path) });
    exes.set(name, list);
  }
  return exes;
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

  constructor(private readonly opts: SweepOptions) {}

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
        this.opts.onRun?.(`${crate}: ${t.target}`);
        const one = await capture([t.exe], t.dir, { CARGO_MANIFEST_DIR: t.dir });
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
      return capture(["cargo", "build", "--release", "-p", "nvs-cli"]);
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
      const r = await this.once(`cmd\0${cwd}\0${argv.join("\0")}`, argv.join(" "), () => capture(argv, join(ROOT, cwd)));
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
      else r = await this.once(`cargo\0${args.join("\0")}`, `cargo ${args.join(" ")}`, () => capture(["cargo", ...args]));
    }
    return judgeTests(c, r, label, (rel) => existsSync(join(ROOT, rel)));
  }
}
