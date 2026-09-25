// Running the proofs on disk. An example runs and its standard output is compared with the `.out` frozen
// beside it. An attack runs and passes when the runtime survived it, whatever the program's own fate.
// `bun nv proofs --run` and `--verify` call this, over any number of groups in one pass: the files of
// every group go into one pool, and each group's verdict lines are counted from its own files.
//
// The programs run on the proof binary, `target/proof/`, and `proofBinary` builds it when the Stage 6 key
// of what it is compiled from has moved since it was last built. The key is the one `builtFrom` computes
// for every Rust build, at the `shipped` tier because the roster reads the binary's own cards. So a relink
// that changed nothing runs nothing again, and an edit to a reference chapter the binary embeds rebuilds.
// `releaseBinary` is the same rule for `target/release/`, which the perf ledger is measured on.
//
// A green verdict is remembered in `.loop/proofs-green.json`, keyed on the program's bytes (with its `.out`
// and `.in`) and on the binary's key, so an unchanged program on an unchanged binary is not run again. Only
// a pass is remembered: a failure is run and reported every time.
//
// A run over whole groups records each group's example and attack directories and bench file in
// `.loop/proof-reads.json`, which is what `tools/nv/keys/checks.ts` keys a `proofs: <group>` unit on.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname } from "node:path";
import { PROOF_READS } from "../keys/checks.ts";
import { metadata } from "../keys/graph.ts";
import { builtFrom, keyOf } from "../keys/key.ts";
import { digest } from "../keys/scan.ts";
import { Tree } from "../keys/tree.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { linked } from "../lib/relink.ts";
import { gapTitle, knownGap } from "./collect.ts";
import { read } from "./roster.ts";

export type What = "examples" | "hostile";
export type Verdict = "ok" | "skip" | "known" | "fail";

export interface Binary {
  path: string;
  /** What a green verdict is remembered against: the Stage 6 key, or the bytes of a binary named by hand. */
  key: string;
}

const EXE = process.platform === "win32" ? "nvs.exe" : "nvs";
export const PROOF_BINARY = `target/proof/${EXE}`;
export const RELEASE_BINARY = `target/release/${EXE}`;
const GREEN = ".loop/proofs-green.json";

/** The two builds a proof runs on. The release build is the argv `bun nv loop`'s acceptance sweep runs, so both share one set
 * of artefacts under `target/release/`. Each writes the key it was built at to `key` beside it. */
const BUILDS = {
  proof: { name: "proof binary", path: PROOF_BINARY, key: "target/proof/nvs.key", argv: ["cargo", "build", "--profile", "proof", "--bin", "nvs"] },
  release: { name: "release binary", path: RELEASE_BINARY, key: "target/release/nvs.key", argv: ["cargo", "build", "--release", "-p", "nvs-cli"] },
};

/** `// requires: unimplemented`, the website's own skip marker. */
const UNIMPL_RE = /^(?:\/\/|#)\s*requires:\s*unimplemented/m;
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

/** The proof binary of the tree as it stands, built first when its key has moved. A string is why not. */
export const proofBinary = () => builtBinary("proof");

/** The release binary of the tree as it stands, which `--record-perf` measures on. */
export const releaseBinary = () => builtBinary("release");

async function builtBinary(profile: keyof typeof BUILDS): Promise<Binary | string> {
  const build = BUILDS[profile];
  const graph = await metadata();
  if (!graph) return `\`cargo metadata\` failed, and the ${build.name}'s key needs the graph`;
  const tree = await Tree.read();
  const key = keyOf(build.name, builtFrom(tree, graph, { own: ["nvs-cli"], ownTier: "shipped", depTier: "shipped", test: false }));
  tree.save();
  const path = abs(build.path);
  const stamp = existsSync(abs(build.key)) ? readFileSync(abs(build.key), "utf8").trim() : "";
  if (existsSync(path) && stamp === key) return { path, key };
  const command = build.argv.join(" ");
  console.error(`nv proofs: building the ${build.name} (\`${command}\`)`);
  const built = await linked(path, () => runProc(build.argv, { timeoutMs: 60 * 60 * 1000 }), (r) => r.stderr);
  if (built.code !== 0 || !existsSync(path)) {
    const tail = built.stderr.trimEnd().split("\n").slice(-15).join("\n");
    return `\`${command}\` failed (exit ${built.code}):\n${tail}`;
  }
  writeFileSync(abs(build.key), `${key}\n`);
  return { path, key };
}

/** A binary named with `--nvs`, taken as it is and remembered by its bytes. */
export function namedBinary(path: string): Binary {
  return { path, key: `bytes:${digest(readFileSync(path))}` };
}

export interface Ran {
  code: number;
  stdout: string;
  stderr: string;
  timedOut: boolean;
  ms: number;
}

const sibling = (proof: string, suffix: string) => proof.replace(/\.nvs$/, suffix);

/** One run of a proof program from the repository root. A proof with a `<name>.in` beside it reads that
 * file as its standard input, and every other proof reads nothing, so no proof ever waits on a terminal. */
export async function spawnProof(argv: string[], proof: string, timeoutMs: number): Promise<Ran> {
  const feed = abs(sibling(proof, ".in"));
  const started = performance.now();
  const child = Bun.spawn(argv, {
    cwd: ROOT,
    stdin: existsSync(feed) ? Bun.file(feed) : "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    child.kill();
  }, timeoutMs);
  try {
    const [stdout, stderr, exited] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
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

/** An example passes when it ends with the status it declares and prints exactly its `.out`. */
async function runExample(nvs: string, path: string): Promise<[Verdict, string]> {
  const source = read(path);
  if (UNIMPL_RE.test(source)) return ["skip", "marked `requires: unimplemented`"];
  const out = await spawnProof([nvs, "run", path], path, EXAMPLE_TIMEOUT_MS);
  if (out.timedOut) return ["fail", `timed out after ${EXAMPLE_TIMEOUT_MS / 1000}s`];
  const want = declaredExit(source);
  if (out.code !== want) {
    const first = out.stderr.trim().split(/\r?\n/)[0] ?? "";
    return ["fail", want ? `exit ${out.code} where it declares \`proof: exit ${want}\`` : `exit ${out.code}: ${first}`];
  }
  const expected = sibling(path, ".out");
  if (!existsSync(abs(expected))) return ["fail", `no ${basename(expected)} beside it`];
  if (normalise(out.stdout) !== normalise(read(expected))) return ["fail", "stdout differs from its .out"];
  return ["ok", ""];
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
async function runHostile(nvs: string, path: string, valgrind: boolean): Promise<[Verdict, string]> {
  const source = read(path);
  if (UNIMPL_RE.test(source)) return ["skip", "marked `requires: unimplemented`"];
  let limit = hostileLimitMs(source);
  let argv = [nvs, "run", path];
  if (valgrind) {
    argv = ["valgrind", "--quiet", "--error-exitcode=97", "--leak-check=full", "--errors-for-leak-kinds=definite", ...argv];
    limit *= 20;
  }
  let out: Ran;
  try {
    out = await spawnProof(argv, path, limit);
  } catch (e) {
    return ["fail", `could not run: ${e instanceof Error ? e.message : String(e)}`];
  }
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

/** What a green verdict on `proof` is remembered against: its bytes, and its `.out` and `.in` when present. */
function proofDigest(proof: string): string {
  const chunks: (string | Uint8Array)[] = [readFileSync(abs(proof))];
  for (const suffix of [".out", ".in"]) {
    const s = abs(sibling(proof, suffix));
    chunks.push(existsSync(s) ? readFileSync(s) : "");
  }
  return digest(...chunks);
}

function loadGreen(): Record<string, [string, string]> {
  try {
    return JSON.parse(readFileSync(abs(GREEN), "utf8"));
  } catch {
    return {};
  }
}

function saveGreen(green: Record<string, [string, string]>): void {
  try {
    mkdirSync(dirname(abs(GREEN)), { recursive: true });
    writeFileSync(abs(GREEN), JSON.stringify(green, Object.keys(green).sort(), 0));
  } catch {
    // A cache that cannot be written is a slow run, never a failed one.
  }
}

/** Records each group's own paths in `PROOF_READS`, which a `proofs: <group>` unit keys on. Groups this
 * run did not name keep their record. */
export function saveReads(groups: [string, string[]][]): void {
  if (groups.length === 0) return;
  let reads: Record<string, string[]> = {};
  try {
    reads = JSON.parse(readFileSync(abs(PROOF_READS), "utf8"));
  } catch {
    // No record yet, or one that cannot be read: this run writes it afresh.
  }
  for (const [group, paths] of groups) reads[group] = paths;
  try {
    mkdirSync(dirname(abs(PROOF_READS)), { recursive: true });
    writeFileSync(abs(PROOF_READS), JSON.stringify(Object.fromEntries(Object.entries(reads).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)))));
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
  cache: boolean;
  strict: boolean;
}

export interface Result {
  verdict: Verdict;
  why: string;
  cached: boolean;
}

export interface Pass {
  /** By `<what>:<path>`. */
  results: Map<string, Result>;
  width: number;
  seconds: number;
}

/** Runs every program named, once each, in one pool. */
export async function runPrograms(bin: Binary, programs: { what: What; path: string }[], opts: RunOptions): Promise<Pass> {
  const green = opts.cache ? loadGreen() : {};
  const results = new Map<string, Result>();
  const todo: { what: What; path: string; slot: string; digest: string }[] = [];
  const seen = new Set<string>();
  for (const { what, path } of programs) {
    const id = `${what}:${path}`;
    if (seen.has(id)) continue;
    seen.add(id);
    const slot = `${what}:${what === "hostile" && opts.valgrind ? "valgrind" : "plain"}:${path}`;
    const d = proofDigest(path);
    const was = green[slot];
    if (was && was[0] === d && was[1] === bin.key) results.set(id, { verdict: "ok", why: "", cached: true });
    else todo.push({ what, path, slot, digest: d });
  }
  const width = jobsFor(todo.length);
  const started = performance.now();
  let next = 0;
  const worker = async () => {
    while (next < todo.length) {
      const t = todo[next++]!;
      const [raw, rawWhy] = t.what === "examples" ? await runExample(bin.path, t.path) : await runHostile(bin.path, t.path, opts.valgrind);
      const [verdict, why] = judgeGap(t.path, raw, rawWhy);
      results.set(`${t.what}:${t.path}`, { verdict, why, cached: false });
      if (verdict === "ok") green[t.slot] = [t.digest, bin.key];
      else delete green[t.slot];
    }
  };
  await Promise.all(Array.from({ length: width }, worker));
  if (opts.cache && todo.length > 0) saveGreen(green);
  return { results, width, seconds: (performance.now() - started) / 1000 };
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
      `(${files.length} files, ${cached} unchanged since they last passed, ${pass.width} at a time, ${pass.seconds.toFixed(1)}s)`,
  );
  return bad.length > 0;
}
