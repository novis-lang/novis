// Running the proofs on disk. An example runs and its standard output is compared with the `.out` frozen
// beside it. An attack runs and passes when the runtime survived it, whatever the program's own fate.
// `bun nv proofs --run` and `--verify` call this, over any number of groups in one pass: the files of
// every group go into one pool, and each group's verdict lines are counted from its own files.
//
// A verdict is taken on the proof binary, `target/proof/`, the optimized `proof` profile built without
// coverage counters: counters slow optimized code several times over, and an attack's time limit is
// written for the binary a person ships. `proofBinary` runs cargo every time, and cargo rebuilds when
// what the binary is built from changed. `releaseBinary` is `target/release/`, which the perf ledger
// measures on, and it too is cargo's to bring up to date: a build that has nothing to do costs cargo's
// own look at its fingerprints.
//
// Which programs run is the selection's (`tools/nv/select/`), and `cmd/proofs.ts` makes it; this file
// runs every program it is handed. What a program used is recorded in a second run of it, on the covws
// debug `nvs` (`recordingRun`), whose outcome is never a verdict. With `NV_PROOF_RECORD=<dir>` in the
// environment each run is recorded into that directory under the program's `recordName`:
// `proofRecording` owns what the program's processes are told. Without it, an instrumented binary's
// counters go to `DISCARD_PROFILE`, never into the working directory.
//
// A run over whole groups records each group's example and attack directories and bench file in the
// selection store (`proofReadsSlot`), which is how `select/checks.ts` knows which proof programs a
// `bun nv proofs --group <group>` check is made of.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { digest } from "../keys/scan.ts";
import { abs, DISCARD_PROFILE, ROOT } from "../lib/paths.ts";
import { killTree, reapOrphans, run as runProc } from "../lib/proc.ts";
import { inPart } from "../lib/reads.ts";
import { proofReadsSlot, SelectStore } from "../select/store.ts";
import { cargoLines, progress } from "../lib/progress.ts";
import { linked } from "../lib/relink.ts";
import { gapTitle, knownGap } from "./collect.ts";
import { read } from "./roster.ts";

export type What = "examples" | "hostile";
export type Verdict = "ok" | "skip" | "known" | "fail";

export interface Binary {
  path: string;
  /** Which build the binary is, or the bytes of a binary named by hand. */
  key: string;
  /** The same, for what a run of it depends on. */
  runs: string;
  /** Whether the selection picks and records the programs judged on it: the proof binary's are, and a
   * binary named by hand runs every program and records nothing. */
  recorded?: boolean;
}

const EXE = process.platform === "win32" ? "nvs.exe" : "nvs";
/** The proof binary, repo-relative: the `proof` profile, built without coverage counters. */
export const PROOF_BINARY = `target/proof/${EXE}`;
export const RELEASE_BINARY = `target/release/${EXE}`;

/** The two optimized builds. The release build is the argv `bun nv loop`'s acceptance sweep runs, so
 * both share one set of artefacts under `target/release/`. */
export const PROOF_BUILD = { name: "proof binary", key: "proof", path: PROOF_BINARY, argv: ["cargo", "build", "--profile", "proof", "--bin", "nvs"] };
const RELEASE = { name: "release binary", key: "release", path: RELEASE_BINARY, argv: ["cargo", "build", "--release", "-p", "nvs-cli"] };

/** How many times its own limit a recording run may take before it counts as hung: the covws debug
 * build is unoptimized and instrumented, so it is many times slower than the binary a verdict is
 * taken on. The same factor the valgrind run gets. */
export const HANG_FACTOR = 20;

/** `// requires: unimplemented`, the website's own skip marker. */
const UNIMPL_RE = /^(?:\/\/|#)\s*requires:\s*unimplemented/m;
/** `// requires: unix`: the program needs what this build has only on Unix — a Unix-domain socket, or a load average. */
const UNIX_RE = /^(?:\/\/|#)\s*requires:\s*unix\b/m;

/** Why a program is not run on this host, or null when it is. A skip is never remembered as green. */
export function skipReason(source: string): string | null {
  if (UNIMPL_RE.test(source)) return "marked `requires: unimplemented`";
  if (UNIX_RE.test(source) && process.platform === "win32") return "marked `requires: unix`, and this host is Windows";
  return null;
}
/** `// hostile: timeout-ms 4000`. */
const TIMEOUT_RE = /(?:\/\/|#)\s*hostile:\s*timeout-ms\s+([0-9]+)/;
/** `// hostile: expect-refusal`: this attack passes when the compiler refuses it. */
const REFUSAL_EXPECTED_RE = /(?:\/\/|#)\s*hostile:\s*expect-refusal/;
/** `// hostile: ends-early`: this attack's last step ends the program, so a non-zero status is expected. */
const ENDS_EARLY_RE = /(?:\/\/|#)\s*hostile:\s*ends-early/;
/** `// proof: exit 1` in an example: the exact status it ends with. */
const EXAMPLE_EXIT_RE = /(?:\/\/|#)\s*proof:\s*exit\s+([0-9]+)/;
/** A compile diagnostic. An uncaught throw is a log line and does not look like this. */
const REFUSED_RE = /^error\[E[0-9]+\]/m;
/** What an attack's output must never carry: the runtime under the program came apart. */
const CRASH_MARKERS = [
  "panicked at",
  "internal error",
  "RUST_BACKTRACE",
  "note: run with",
  "double free",
  "Assertion failed",
  "AddressSanitizer",
  "SIGSEGV",
  "stack overflow",
];
const EXAMPLE_TIMEOUT_MS = 60_000;
export const HOSTILE_TIMEOUT_MS = 10_000;

/** The proof binary of the tree as it stands, uninstrumented, which verdicts are taken on. A string is
 * why there is none. */
export const proofBinary = () => builtBinary(PROOF_BUILD, true);

/** The release binary of the tree as it stands, which `--record-perf` measures on. */
export const releaseBinary = () => builtBinary(RELEASE, false);

async function builtBinary(build: typeof RELEASE, recorded: boolean): Promise<Binary | string> {
  progress(`proofs: bringing the ${build.name} up to date`);
  const path = abs(build.path);
  const command = build.argv.join(" ");
  const onLine = cargoLines(`proofs: building the ${build.name}`);
  const built = await linked(path, () => runProc(build.argv, { timeoutMs: 60 * 60 * 1000, onLine }), (r) => r.stderr);
  if (built.code !== 0 || !existsSync(path)) {
    const tail = built.stderr.trimEnd().split("\n").slice(-15).join("\n");
    return `\`${command}\` failed (exit ${built.code}):\n${tail}`;
  }
  return { path, key: build.key, runs: build.key, ...(recorded ? { recorded } : {}) };
}

/** A binary named with `--nvs`, taken as it is and remembered by its bytes. */
export function namedBinary(path: string): Binary {
  const key = `bytes:${digest(readFileSync(path))}`;
  return { path, key, runs: key };
}

export interface Ran {
  code: number;
  stdout: string;
  stderr: string;
  timedOut: boolean;
  ms: number;
}

const sibling = (proof: string, suffix: string) => proof.replace(/\.nvs$/, suffix);

/** The environment variable naming the directory each proof program's run is recorded into. */
export const RECORD_ENV = "NV_PROOF_RECORD";

/**
 * The file name a recorded program's files start with, from its repo-relative path: `/` and `\` become
 * `~`, and every other byte that is not an ASCII letter, digit, `.`, `_` or `-` becomes `@` and two hex
 * digits. A name longer than 96 bytes keeps its first 64 and ends in `@` and the 64-bit FNV-1a hash of
 * the whole name as sixteen hex digits, so a profile's path stays inside Windows' 260-character limit.
 * The same rule as `nvs test --record`'s (`nvs_test::record_name`), so one reader serves both.
 */
export function recordName(path: string): string {
  let name = "";
  for (const byte of new TextEncoder().encode(path)) {
    const c = String.fromCharCode(byte);
    if (c === "/" || c === "\\") name += "~";
    else if (/[A-Za-z0-9._-]/.test(c)) name += c;
    else name += `@${byte.toString(16).padStart(2, "0")}`;
  }
  if (name.length <= 96) return name;
  let hash = 0xcbf29ce484222325n;
  for (let i = 0; i < name.length; i++) {
    hash ^= BigInt(name.charCodeAt(i));
    hash = (hash * 0x100000001b3n) & 0xffffffffffffffffn;
  }
  return `${name.slice(0, 64)}@${hash.toString(16).padStart(16, "0")}`;
}

/**
 * The variables every process of one run of `proof` gets when `dir` names a record directory:
 * `LLVM_PROFILE_FILE` as `<dir>/<name>-%p.profraw`, one file per process, `NVS_FOOTPRINT_LOG` as
 * `<dir>/<name>.log`, which they share, and `NOVIS_NO_FILE_CACHE`, so the compile runs whole. A
 * relative `dir` is taken from the repository root. Empty when nothing is recorded.
 */
export function proofRecording(proof: string, dir: string | undefined = process.env[RECORD_ENV]): Record<string, string> {
  if (!dir) return {};
  const at = resolve(ROOT, dir);
  const name = recordName(proof);
  return {
    LLVM_PROFILE_FILE: resolve(at, `${name}-%p.profraw`),
    NVS_FOOTPRINT_LOG: resolve(at, `${name}.log`),
    NOVIS_NO_FILE_CACHE: "1",
  };
}

/** One run of a proof program from the repository root. A proof with a `<name>.in` beside it reads that
 * file as its standard input, and every other proof reads nothing, so no proof ever waits on a terminal.
 * A proof with a `<name>.nvsr` beside it answers the request that file describes, handed over as
 * `--request` after `run`, so a `Core\Request` member has a request to read; every other proof answers
 * none, and a `Core\Request` member there throws. A proof whose directory holds an `nvs.toml` runs
 * under that file alone, handed over as `--config` after `run`, so a feature that needs a grant carries
 * it where its reader sees it; every other proof runs under the repository root's. `record` names the
 * directory the run is recorded into, which defaults to `NV_PROOF_RECORD`, and variables its processes
 * get beside the recording's. With `unlogged` and no record directory, the run writes no footprint log,
 * not even the one this process inherited. */
export async function spawnProof(argv: string[], proof: string, timeoutMs: number, record?: { dir?: string; env?: Record<string, string>; unlogged?: boolean }): Promise<Ran> {
  const feed = abs(sibling(proof, ".in"));
  const request = sibling(proof, ".nvsr");
  const config = `${dirname(proof)}/nvs.toml`;
  const at = argv.indexOf("run");
  if (at >= 0 && existsSync(abs(request))) argv = [...argv.slice(0, at + 1), "--request", request, ...argv.slice(at + 1)];
  if (at >= 0 && existsSync(abs(config))) argv = [...argv.slice(0, at + 1), "--config", config, ...argv.slice(at + 1)];
  const recording = proofRecording(proof, record?.dir ?? process.env[RECORD_ENV]);
  if (recording.NVS_FOOTPRINT_LOG) mkdirSync(dirname(recording.NVS_FOOTPRINT_LOG), { recursive: true });
  const env: Record<string, string | undefined> = { LLVM_PROFILE_FILE: DISCARD_PROFILE, ...process.env, ...record?.env, ...recording };
  if (record?.unlogged && !recording.NVS_FOOTPRINT_LOG) delete env.NVS_FOOTPRINT_LOG;
  const started = performance.now();
  const child = Bun.spawn(argv, {
    cwd: ROOT,
    env,
    stdin: existsSync(feed) ? Bun.file(feed) : "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    killTree(child.pid);
  }, timeoutMs);
  try {
    const reading = Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text()]);
    const exited = await child.exited;
    // What a failed or killed program left running is killed with it, so it holds no pipe and no file.
    if (exited !== 0 || timedOut) reapOrphans(child.pid);
    const [stdout, stderr] = await reading;
    // A program ended by a signal has no exit status of its own, and that is crash-shaped.
    const code = child.signalCode && !timedOut ? -1 : exited;
    return { code, stdout, stderr, timedOut, ms: performance.now() - started };
  } finally {
    clearTimeout(timer);
  }
}

const normalise = (text: string) => text.replace(/\r\n/g, "\n").trimEnd();

/** The status an example says it ends with, and 0 for every example that says nothing. */
function declaredExit(source: string): number {
  const m = EXAMPLE_EXIT_RE.exec(source);
  return m ? Number(m[1]) : 0;
}

/** One judged run: the verdict, why, and what the program did, which a program skipped on this host
 * does not have. */
type Judged = [Verdict, string, Ran?];

/** An example passes when it ends with the status it declares and prints exactly its `.out`. */
async function runExample(nvs: string, path: string, unlogged = false): Promise<Judged> {
  const source = read(path);
  const skip = skipReason(source);
  if (skip) return ["skip", skip];
  const out = await spawnProof([nvs, "run", path], path, EXAMPLE_TIMEOUT_MS, unlogged ? { unlogged } : undefined);
  if (out.timedOut) return ["fail", `timed out after ${EXAMPLE_TIMEOUT_MS / 1000}s`, out];
  const want = declaredExit(source);
  if (out.code !== want) {
    const first = out.stderr.trim().split(/\r?\n/)[0] ?? "";
    return ["fail", want ? `exit ${out.code} where it declares \`proof: exit ${want}\`` : `exit ${out.code}: ${first}`, out];
  }
  const expected = sibling(path, ".out");
  if (!existsSync(abs(expected))) return ["fail", `no ${basename(expected)} beside it`, out];
  if (normalise(out.stdout) !== normalise(read(expected))) return ["fail", "stdout differs from its .out", out];
  return ["ok", "", out];
}

/**
 * `--bless`: writes each example's `.out` from what it prints, and prints what it wrote. This is how an
 * expected output is created, never how a red example is made green, which is why the output is shown
 * rather than written silently. An example that does not end with the status it declares gets no `.out`.
 * Returns the lines to print and whether any example failed.
 */
export async function bless(nvs: string, paths: string[]): Promise<{ lines: string[]; failed: boolean }> {
  const lines: string[] = [];
  let failed = false;
  for (const path of paths) {
    const skip = skipReason(read(path));
    if (skip) {
      // What a skipped program prints here is not what it prints where it runs.
      lines.push(`  FAIL  ${path} is ${skip}: bless it on a host that runs it`);
      failed = true;
      continue;
    }
    const out = await spawnProof([nvs, "run", path], path, 120_000);
    if (out.code !== declaredExit(read(path))) {
      lines.push(`  FAIL  ${path} exited ${out.code}:`, "        " + out.stderr.trim().replace(/\r?\n/g, "\n        "));
      failed = true;
      continue;
    }
    const dest = sibling(path, ".out");
    const existed = existsSync(abs(dest));
    const text = out.stdout.replace(/\r\n/g, "\n");
    writeFileSync(abs(dest), text);
    lines.push(`  ${existed ? "rewrote" : "wrote"}  ${dest}`);
    for (const line of text.trimEnd().split("\n")) lines.push(`      | ${line}`);
  }
  if (!failed) lines.push("nv proofs: read what was written -- a blessed output is a claim, not a formality.");
  return { lines, failed };
}

/** How far an attack got before its clock ran out, from the lines its steps printed. */
function reached(stdout: string): string {
  const done = stdout.split(/\r?\n/).filter((l) => l.trim());
  if (done.length === 0) return ", having printed nothing";
  return `, having printed ${done.length} line(s), the last ${JSON.stringify([...done.at(-1)!].slice(0, 60).join(""))}`;
}

/** An attack's time limit: its `timeout-ms`, or the default. */
export function hostileLimitMs(source: string): number {
  const m = TIMEOUT_RE.exec(source);
  return m ? Number(m[1]) : HOSTILE_TIMEOUT_MS;
}

/**
 * An attack passes when the runtime survives it. A throw, a limit, a clean fatal and a clean run all pass.
 * A panic, a hang, a crash-shaped status and a definite leak do not. Two failures look like a pass, and
 * each is an attack that was never delivered: a compile diagnostic, unless the file declares
 * `expect-refusal`, and a program that ends before its last line, unless the file declares `ends-early`.
 */
async function runHostile(nvs: string, path: string, valgrind: boolean, unlogged = false): Promise<Judged> {
  const source = read(path);
  const skip = skipReason(source);
  if (skip) return ["skip", skip];
  let limit = hostileLimitMs(source);
  let argv = [nvs, "run", path];
  if (valgrind) {
    argv = ["valgrind", "--quiet", "--error-exitcode=97", "--leak-check=full", "--errors-for-leak-kinds=definite", ...argv];
    limit *= HANG_FACTOR;
  }
  let out: Ran;
  try {
    out = await spawnProof(argv, path, limit, unlogged ? { unlogged } : undefined);
  } catch (e) {
    return ["fail", `could not run: ${e instanceof Error ? e.message : String(e)}`];
  }
  return [...judgeHostile(source, out, limit, valgrind), out];
}

/** An attack's verdict from one run of it under `limit`. */
function judgeHostile(source: string, out: Ran, limit: number, valgrind: boolean): [Verdict, string] {
  if (out.timedOut) return ["fail", `still running after ${Math.round(limit / 1000)}s -- unbounded${reached(out.stdout)}`];
  const blob = out.stderr + out.stdout;
  for (const marker of CRASH_MARKERS) if (blob.includes(marker)) return ["fail", `stderr carries ${JSON.stringify(marker)}`];
  const refused = REFUSED_RE.test(out.stderr) && !out.stdout.trim();
  if (REFUSAL_EXPECTED_RE.test(source)) {
    return refused ? ["ok", ""] : ["fail", "declares `expect-refusal`, but the compiler accepted it"];
  }
  if (refused) {
    const first = out.stderr.split(/\r?\n/).find((l) => l.startsWith("error[")) ?? "";
    return ["fail", `never ran -- it does not compile: ${first}`];
  }
  if (valgrind && out.code === 97) return ["fail", "valgrind reports a definite leak or an invalid access"];
  // A negative status is a signal; anything past 255 on Windows is a structured exception.
  if (out.code < 0 || out.code > 255) return ["fail", `crash-shaped exit status ${out.code}`];
  const endsEarly = ENDS_EARLY_RE.test(source);
  if (out.code !== 0 && !endsEarly) {
    const first = out.stderr.split(/\r?\n/).find((l) => l.trim()) ?? "";
    return [
      "fail",
      `ended before its last line (exit ${out.code}), so the steps behind that point never ran -- catch what ` +
        `stopped it, or make that step the last one and declare \`ends-early\`: ${[...first].slice(0, 120).join("")}`,
    ];
  }
  if (out.code === 0 && endsEarly) return ["fail", "declares `ends-early`, but ran to its last line -- remove the marker"];
  return ["ok", ""];
}

/** One verdict re-judged against the file's `proof: gap` marker. The marker must name a gap record. A
 * marked proof that fails is counted as a known gap. A marked proof that passes fails, because the marker
 * has to go with the bug it names. */
function judgeGap(path: string, verdict: Verdict, why: string): [Verdict, string] {
  const id = knownGap(read(path));
  if (!id) return [verdict, why];
  const title = gapTitle(id);
  if (title === null) return ["fail", `\`proof: gap\` names ${id}, which is not a record under data/gaps/`];
  if (verdict === "ok") return ["fail", `passes, but is still marked \`proof: gap ${id}\` -- remove the marker`];
  if (verdict === "skip") return [verdict, why];
  return ["known", `${title} (gap ${id})`];
}

/** Records each group's own paths in the selection store, which a `proofs: <group>` unit keys on.
 * Groups this run did not name keep their record. */
export function saveReads(groups: [string, string[]][]): void {
  if (groups.length === 0) return;
  try {
    const store = new SelectStore();
    try {
      store.transaction(() => {
        for (const [group, paths] of groups) store.putVerdict(proofReadsSlot(group), "", JSON.stringify(paths));
      });
    } finally {
      store.close();
    }
  } catch {
    // A record that cannot be written keys the group on everything, which only re-runs it.
  }
}

/** How many programs run at once: `NVS_PROOF_JOBS`, or half the cores. */
function jobsFor(count: number): number {
  const env = Number(process.env.NVS_PROOF_JOBS);
  const cores = navigator.hardwareConcurrency || 2;
  const width = Number.isInteger(env) && env > 0 ? env : Math.max(1, Math.floor(cores / 2));
  return Math.max(1, Math.min(width, count));
}

export interface RunOptions {
  valgrind: boolean;
  strict: boolean;
  /** The judged runs write no footprint log, not even one this process inherited: the selection records
   * each program on its own recording run instead. */
  unlogged?: boolean;
}

export interface Result {
  verdict: Verdict;
  why: string;
  /** Not run: the change reaches nothing it ran, and its last run was green. */
  cached: boolean;
  /** What the judged run did; absent for a program not run or skipped on this host. */
  ran?: Ran;
}

export interface Pass {
  /** By `<what>:<path>`. */
  results: Map<string, Result>;
  width: number;
  seconds: number;
}

/** A program to run, and the parts of the process its reads are noted under (`inPart`); none notes them
 * for the process as a whole. */
export interface Program {
  what: What;
  path: string;
  parts?: readonly string[];
}

/** Runs every program named, once each, in one pool; each of `unchanged` is reported green without a
 * run. */
export async function runPrograms(bin: Binary, programs: Program[], opts: RunOptions, unchanged: Set<string> = new Set()): Promise<Pass> {
  const results = new Map<string, Result>();
  const todo: Program[] = [];
  const seen = new Set<string>();
  for (const p of programs) {
    const id = `${p.what}:${p.path}`;
    if (seen.has(id)) continue;
    seen.add(id);
    if (unchanged.has(p.path)) results.set(id, { verdict: "ok", why: "", cached: true });
    else todo.push(p);
  }
  const width = jobsFor(todo.length);
  const started = performance.now();
  const cached = results.size;
  let next = 0;
  let ran = 0;
  let failed = 0;
  const say = () => progress(`proofs: ${ran}/${todo.length} programs run${cached ? ` (${cached} more unchanged)` : ""}${failed ? `, ${failed} failed` : ""}`);
  if (todo.length > 0) say();
  const worker = async () => {
    while (next < todo.length) {
      const t = todo[next++]!;
      const unlogged = opts.unlogged === true;
      const [verdict, why, out] = await inPart(t.parts ?? [], async () => {
        const [raw, rawWhy, ran] = t.what === "examples" ? await runExample(bin.path, t.path, unlogged) : await runHostile(bin.path, t.path, opts.valgrind, unlogged);
        return [...judgeGap(t.path, raw, rawWhy), ran] as const;
      });
      results.set(`${t.what}:${t.path}`, { verdict, why, cached: false, ...(out ? { ran: out } : {}) });
      ran++;
      if (verdict === "fail") failed++;
      say();
    }
  };
  await Promise.all(Array.from({ length: width }, worker));
  return { results, width, seconds: (performance.now() - started) / 1000 };
}

/** How long a recording run of `path` may take before it is killed as hung: its own limit times
 * `HANG_FACTOR`, or its own limit alone when the judged run already ran out of it. */
export function recordingLimitMs(what: What, path: string, judged?: Ran): number {
  const limit = what === "hostile" ? hostileLimitMs(read(path)) : EXAMPLE_TIMEOUT_MS;
  return judged?.timedOut ? limit : limit * HANG_FACTOR;
}

/** The recording run of one program on `nvs`, the covws debug build, recorded into `dir` with `env`
 * beside it. It has a hang limit and no verdict. */
export function recordingRun(nvs: string, what: What, path: string, dir: string, env: Record<string, string> = {}, judged?: Ran): Promise<Ran> {
  return spawnProof([nvs, "run", path], path, recordingLimitMs(what, path, judged), { dir, env });
}

/**
 * Why a recording run ended differently from the judged run, or null when both ended alike: the same
 * exit status and the same standard output. A recording run that ends differently may have stopped
 * short of code the judged run reached, so its footprint is not trusted. A judged run that ran out of
 * time is red whatever the recording run did, and is compared with nothing.
 */
export function divergence(judged: Ran, recorded: Ran): string | null {
  if (judged.timedOut) return null;
  if (recorded.timedOut) return `the recording run was still running after ${Math.round(recorded.ms / 1000)}s`;
  if (recorded.code !== judged.code) return `exit ${recorded.code} on the recording run, ${judged.code} on the judged run`;
  const a = normalise(judged.stdout).split("\n");
  const b = normalise(recorded.stdout).split("\n");
  if (a.join("\n") === b.join("\n")) return null;
  let line = 0;
  while (line < a.length && line < b.length && a[line] === b[line]) line++;
  return `stdout differs from the judged run's at line ${line + 1}`;
}

/** One program run for a person to read: a `== <path>` line, what it printed, then its exit status and
 * time. An attack's time is shown against its limit. Nothing is judged here and nothing is remembered. */
export async function showProgram(nvs: string, path: string, kind: What | "bench"): Promise<string> {
  const limit = kind === "hostile" ? hostileLimitMs(read(path)) : EXAMPLE_TIMEOUT_MS;
  const out = await spawnProof([nvs, "run", path], path, limit);
  const lines = [`== ${path}`];
  const printed = (out.stdout + out.stderr).replace(/\r\n/g, "\n").trimEnd();
  if (printed) lines.push(printed);
  const ms = Math.round(out.ms);
  const status = out.timedOut ? "killed, out of time" : `exit ${out.code}`;
  lines.push(kind === "hostile" ? `${status} · ${ms} ms of ${limit} ms` : `${status} · ${ms} ms`);
  return `${lines.join("\n")}\n\n`;
}

/** One suite's verdict lines over `files`, out of a pass that ran them. Returns whether any failed. */
export function suiteLines(out: string[], what: What, files: string[], pass: Pass, opts: RunOptions & { quiet: boolean }): boolean {
  if (files.length === 0) {
    out.push(`proofs ${what}: 0 ok, 0 skipped, 0 known-gap, 0 failed (no ${what} on disk yet -- nothing to run)`);
    return false;
  }
  const rows = files.map((f) => [f, pass.results.get(`${what}:${f}`)!] as const);
  const ok = rows.filter(([, r]) => r.verdict === "ok").length;
  const cached = rows.filter(([, r]) => r.cached).length;
  const skipped = rows.filter(([, r]) => r.verdict === "skip");
  let gaps = rows.filter(([, r]) => r.verdict === "known");
  const bad = rows.filter(([, r]) => r.verdict === "fail");
  if (opts.strict) {
    bad.push(...gaps);
    gaps = [];
  }
  const byPath = (a: readonly [string, Result], b: readonly [string, Result]) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0);
  for (const [f, r] of bad.sort(byPath)) out.push(`  FAIL  ${f}: ${r.why}`);
  for (const [f, r] of gaps.sort(byPath)) out.push(`  gap   ${f}: ${r.why}`);
  if (!opts.quiet) for (const [f, r] of skipped.sort(byPath)) out.push(`  skip  ${f}: ${r.why}`);
  out.push(
    `proofs ${what}: ${ok} ok, ${skipped.length} skipped, ${gaps.length} known-gap, ${bad.length} failed ` +
      `(${files.length} files, ${cached} green and not reached by the change, ${pass.width} at a time, ${pass.seconds.toFixed(1)}s)`,
  );
  return bad.length > 0;
}
