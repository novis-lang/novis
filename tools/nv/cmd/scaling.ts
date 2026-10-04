// `bun nv scaling`: runs a program at growing sizes and reports how fast its cost grows. The rule it
// enforces is the user's: linear is correct and a little steeper is fine, and quadratic or steeper is a
// defect. `benches/scaling/README.md` is what a ladder is; this doc is the one home of the ramp, the
// agreement test, the ceiling and the bounds.
//
//     bun nv scaling                             every ladder under benches/scaling/
//     bun nv scaling --iterations                every bench under benches/members/
//     bun nv scaling --iterations core/Str       only the benches whose path contains `core/Str`
//     bun nv scaling --areas [--reviewed]        whether every area in `AREAS` has a ladder (and a review)
//
// `--check` adds one closing line when nothing grows past its bound, which is what an acceptance check
// reads. A run exits 1 when a program grows, and a ladder run also when a ladder is invalid.
//
// `--iterations` runs every bench, `.scale.nvs` and `.twin.nvs` siblings included, at a series of
// small batches, from a copy under `.agent-tmp/` whose closing `echo Bench::run(N)` line names the
// batch instead of the bench's own `iterations N`. The bench's whole folder is copied, and beside the
// copy goes the folder's `nvs.toml`, or the repository's, with its paths rebased so the copy keeps the
// capabilities and the app the original has. A bench whose closing line does not pass its
// `iterations N` as one integer literal, or whose N is too small to ramp, is skipped and named.
//
// The ramp. The first batch is `START` operations and each next batch doubles the one before. A batch
// is one `nvs run --count` (the counting run of `tools/nv/proofs/perf.ts`) and the fastest of `--reps`
// plain runs. A batch's *increment* is what it cost beyond the batch before, per added operation, so
// the program's fixed start-up and set-up cancel out exactly. The ramp stops when the last three
// increments agree, or when the next batch would pass `CEILING` or the bench's own N, whichever is
// lower. No session raises `CEILING`.
//
// The agreement test, judged on the four counts alone and never on the clock, so noise can never push
// a batch up. For each count, three increments d1, d2, d3 give two local slopes, `1 + log2(d2 / d1)`
// and `1 + log2(d3 / d2)`, since cost `n^k` doubles its increment `2^(k-1)` times per doubling. They
// agree when every count's two local slopes differ by at most `AGREE`, or when its three increments
// lie within `FLAT` of each other per operation, which is how a count that does nothing per operation,
// or a rare set-up allocation, settles. A count's slope is `1 + log2(d3 / d1) / 2`.
//
// The bounds. A count whose slope is above `COUNT_BOUND` fails: each operation leaves something behind
// that the next one pays for. The clock's slope is read the same way, from the clock's increments, and
// is reported. Above `CLOCK_BOUND` it counts only when a second run of the same batches agrees, because
// the clock is the second signal and never the first. Even then it does not fail a bench whose counts
// settled within their bound: the second run shares the first one's machine load, so callgrind
// settles it, as below. A clock increment at or under zero leaves the clock's slope unread.
//
// A bench that reaches the ceiling and has not agreed, or whose clock grew twice, runs the same batches again under
// `valgrind --tool=callgrind`, in WSL on Windows, with the Linux binary `--wsl-nvs` names. Callgrind's
// instruction count is nearly the same on every run, so it is judged as one more count. A bench still
// unclear after that is reported under *unclear* and is not judged.
//
// A ladder, under `benches/scaling/<area>/`, ramps its input size the same way: its `// scaling:` lines
// give `start`, `max`, `expect` and optionally `kind` and `proposal`, and its closing line passes
// `start` as the literal the ramp rewrites. The sizes double from `start` up to `max`, and the ramp,
// the agreement test and callgrind are the bench's. `expect` sets the count bound from `EXPECT`, and
// the clock bound sits as far above it as `CLOCK_BOUND` sits above `COUNT_BOUND`. `constant` passes
// only increments that shrink; `quadratic` is never accepted. A ladder that would fail and is marked
// `proposal` is reported as waiting for the user's decision and does not fail.
//
// A ladder's kind says what is measured at each size. `run` is a bench's: `nvs run --count` and the
// clock of `nvs run`. `compile` runs the ladder once to print a program, writes it beside the copy, and
// takes the `compile:` line of `nvs check --count` and the `ir` of `nvs run --count` on it; their sum
// is judged as `total` beside them, and the clock is `nvs check`'s. Callgrind runs `nvs check` on the
// printed program. `fmt` prints a file the same way and has no counts: every size runs, the clock of
// `nvs fmt --check` decides under the second-run bound, and a clock slope left unread goes to callgrind.
// `lsp` prints a document that marks one `<|>` cursor and is judged as `fmt` is, on the clock of one
// `nvs lsp` session over stdio (`lspScript`): open, an edit, then completion, hover and references at
// the cursor, timed from the open to the last answer so the server's start-up is outside it. Callgrind
// runs the same session with its messages piped in from a file.
//
// `serve` is the one kind whose copy is not rewritten: the ladder is the program every request runs,
// and `// scaling: size` names what the size counts (`SERVE_SIZES`, `serveShape`). At each size the
// tool boots `nvs serve` on the copy, on a free port, sends the load with `bun nv bench`'s client, and
// stops the server. `requests`, the default, is that many GETs over one keep-alive connection. `headers`,
// `header-bytes` and `query` send `SHAPED_REQUESTS` GETs that each carry that many extra headers, one
// header of that many bytes, or that many query parameters, the last of them `q`; `body-bytes` sends as
// many POSTs whose body is that many bytes. `connections` opens that many keep-alive connections before
// the clock starts and sends `PER_CONNECTION` requests over each, all at once. The clock is the load's,
// from the first request to the last answer, and the fastest of `--reps` boots. The server prints no
// counts, so the clock decides as it does for `fmt`, and a clock slope left unread is reported invalid,
// because a server is not run under callgrind. Its peak memory, the most the OS saw the server process
// hold, is the lowest of the boots. Under `size requests` it must not grow: a peak at the largest size
// more than `PEAK_SLACK` above the lowest peak of the ramp fails the ladder, because memory is
// O(in-flight) and never O(requests served). Every other size grows what is in flight, so its peak may
// grow with it and is not judged.
//
// `fetch` is `run` with a server to download from. At each size the tool boots `nvs serve` of
// `<name>/server.nvs`, kept beside the ladder, on a free port, names it to the ladder as
// `http://127.0.0.1:<port>` in `FETCH_URL_ENV`, takes `run`'s counts and clock, and stops the server.
// The size is the ladder's closing literal, so the program asks the server for the response it needs:
// `http-client/response.nvs` downloads one body of that many bytes. The server is another process,
// so its work is inside the clock and outside the counts. A `fetch` ladder is not run under callgrind.
//
// The counts are the same on every machine, so a bench gets the same verdict everywhere. The clock is
// taken on the release binary unless `--nvs` names another, as `--record-perf` takes it.

import { cpSync, existsSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { abs, DISCARD_PROFILE, rel, ROOT } from "../lib/paths.ts";
import { progress } from "../lib/progress.ts";
import { fixed } from "../lib/py.ts";
import { countProgram, COUNTS, PerfError } from "../proofs/perf.ts";
import { read } from "../proofs/roster.ts";
import { releaseBinary, skipReason, spawnProof } from "../proofs/run.ts";
import { freePort, hammer, HttpConn, type HttpShape, Server } from "./bench.ts";

export const summary = "how fast a program's cost grows with its size: nv scaling [--iterations] [--check] [path-filter...] | --areas [--reviewed]";

const BENCHES = "benches/members";
const LADDERS = "benches/scaling";
const REVIEW = "docs/perf/performance-review.md";
/** The line `--check` prints when nothing grows past its bound. */
const PASSED_ITERATIONS = "every bench costs the same per operation in every batch";
const PASSED_LADDERS = "every ladder is within its declared growth";
const CALIBRATION = "_calibration";

/** The first batch, in operations. */
export const START = 16;
/** The largest batch any bench reaches. */
export const CEILING = 4096;
/** How far two local slopes of one count may differ and still agree. */
export const AGREE = 0.05;
/** How far apart, per operation, three increments of one count may lie and still agree as flat. */
export const FLAT = 0.01;
/** The steepest a count may grow. */
export const COUNT_BOUND = 1.15;
/** The steepest the clock may grow, when a second run agrees. */
export const CLOCK_BOUND = 1.5;
/** The fewest batches a bench needs: three increments over the batch before each. */
const MIN_BATCHES = 4;
const TIMEOUT_MS = 600_000;
const ITER_RE = /(?:\/\/|#)\s*bench:\s*iterations\s+([0-9_]+)/;
/** `==12345== Collected : 987654321`, the instruction total callgrind prints at exit. */
const COLLECTED_RE = /Collected\s*:\s*(\d+)/;
const DEFAULT_WSL_NVS = "/var/tmp/nvs-target-wsl/debug/nvs";

/** One batch: its size, its totals, and the fastest plain run in nanoseconds. */
export interface Batch {
  size: number;
  counts: Record<string, number>;
  ns: number;
}

const number = (digits: string) => Number(digits.replace(/_/g, ""));

/**
 * The bench's source with its closing `echo Bench::run(...)` line passing `batch` where it passed
 * `iterations`, or null when that line does not pass `iterations` as exactly one integer literal.
 */
export function withBatch(source: string, iterations: number, batch: number): string | null {
  const lines = source.split("\n");
  const at = lines.findIndex((l) => /^echo Bench::run\(/.test(l));
  if (at < 0) return null;
  const line = lines[at]!;
  const open = line.indexOf("(");
  let depth = 0;
  let close = -1;
  for (let i = open; i < line.length; i++) {
    if (line[i] === "(") depth++;
    else if (line[i] === ")" && --depth === 0) {
      close = i;
      break;
    }
  }
  if (close < 0) return null;
  const args = line.slice(open + 1, close);
  const hits = [...args.matchAll(/(?<![\w.$\\])[0-9][0-9_]*(?![\w.])/g)].filter((m) => number(m[0]) === iterations);
  if (hits.length !== 1) return null;
  const from = open + 1 + hits[0]!.index!;
  lines[at] = line.slice(0, from) + String(batch) + line.slice(from + hits[0]![0].length);
  return lines.join("\n");
}

/** The batch sizes a bench of `iterations` operations may run: doubling from `START`, below its own N
 * and at most `CEILING`, started lower when that leaves fewer than `MIN_BATCHES`. Empty when even that
 * does not fit. */
export function batchSizes(iterations: number): number[] {
  let cap = 1;
  while (cap * 2 < iterations && cap * 2 <= CEILING) cap *= 2;
  let start = Math.min(START, cap >> (MIN_BATCHES - 1));
  if (start < 1) return [];
  const sizes: number[] = [];
  for (let s = start; s <= cap; s *= 2) sizes.push(s);
  return sizes;
}

/** Each batch's cost beyond the batch before it, per added operation, for `value`. */
export function increments(batches: Batch[], value: (b: Batch) => number): number[] {
  const out: number[] = [];
  for (let i = 1; i < batches.length; i++) {
    out.push((value(batches[i]!) - value(batches[i - 1]!)) / (batches[i]!.size - batches[i - 1]!.size));
  }
  return out;
}

/** Whether the last three increments agree, by the agreement test above. */
export function agrees(d: number[]): boolean {
  if (d.length < 3) return false;
  const [a, b, c] = d.slice(-3) as [number, number, number];
  if (Math.max(a, b, c) - Math.min(a, b, c) <= FLAT) return true;
  if (a <= 0 || b <= 0 || c <= 0) return false;
  return Math.abs(Math.log2(b / a) - Math.log2(c / b)) <= AGREE;
}

/** The slope the last three increments give, or null when the count does nothing per operation or an
 * increment is not above zero. */
export function slopeOf(d: number[]): number | null {
  if (d.length < 3) return null;
  const [a, b, c] = d.slice(-3) as [number, number, number];
  if (Math.max(Math.abs(a), Math.abs(b), Math.abs(c)) <= FLAT) return null;
  if (a <= 0 || b <= 0 || c <= 0) return null;
  return 1 + Math.log2(c / a) / 2;
}

/** Whether every count's last three increments agree. */
export function countsAgree(batches: Batch[], keys: readonly string[] = COUNTS): boolean {
  return keys.every((k) => agrees(increments(batches, (b) => b.counts[k]!)));
}

/** The counts of the `compile:` line `nvs check --count` prints, in its order; `nvs run --count` adds `ir`. */
export const COMPILE_COUNTS = ["tokens", "nodes", "names", "exprs", "ir"] as const;

/** The counts on the `compile:` line in `stderr`, or null when there is none. */
export function compileCounts(stderr: string): Record<string, number> | null {
  const line = stderr.split(/\r?\n/).find((l) => l.startsWith("compile: "));
  if (!line) return null;
  const pairs = line.slice("compile: ".length).trim().split(/\s+/).map((p) => p.split("="));
  return Object.fromEntries(pairs.filter(([k, v]) => (COMPILE_COUNTS as readonly string[]).includes(k!) && /^\d+$/.test(v ?? "")).map(([k, v]) => [k!, Number(v)]));
}

export type Verdict = "flat" | "grows" | "unclear" | "skipped" | "proposal" | "invalid";

export interface Judged {
  bench: string;
  verdict: Verdict;
  sizes: number[];
  /** Each count's slope, null where it does nothing per operation. */
  slopes: Record<string, number | null>;
  clock: number | null;
  /** Why it failed, was not judged, or was skipped. */
  notes: string[];
}

/** The verdict on a finished ramp's counts: which count grows past the bound, and by how much. */
export function judgeCounts(batches: Batch[], keys: readonly string[] = COUNTS, bound = COUNT_BOUND): { slopes: Record<string, number | null>; over: string[] } {
  const slopes: Record<string, number | null> = {};
  const over: string[] = [];
  for (const k of keys) {
    const d = increments(batches, (b) => b.counts[k]!);
    const s = slopeOf(d);
    slopes[k] = s;
    if (s !== null && s > bound) {
      over.push(`${k} grows with slope ${fixed(s, 2)}, ${fixed(d.at(-3)!, 3)} per op at ${batches.at(-3)!.size} and ${fixed(d.at(-1)!, 3)} at ${batches.at(-1)!.size}`);
    }
  }
  return { slopes, over };
}

const clockSlope = (batches: Batch[]) => slopeOf(increments(batches, (b) => b.ns));

/** The steepest a ramp's counts and its clock may grow. */
export interface Bounds {
  count: number;
  clock: number;
}

const BENCH_BOUNDS: Bounds = { count: COUNT_BOUND, clock: CLOCK_BOUND };

/** What each `// scaling: expect` allows a count to grow by; the clock gets the same margin above it
 * that `CLOCK_BOUND` has above `COUNT_BOUND`. */
export const EXPECT: Record<string, number> = { constant: 0.15, linear: COUNT_BOUND, nlogn: 1.35, karatsuba: 1.7 };

/** The kinds a ladder may declare. */
export const KINDS = ["run", "compile", "lsp", "fmt", "serve", "fetch"] as const;

/** The variable a `fetch` ladder reads its server's address from. */
export const FETCH_URL_ENV = "NVS_SCALING_URL";

/** How far a `serve` ramp's peak memory may rise above the lowest peak it reached, as a fraction of that peak. */
export const PEAK_SLACK = 0.25;

/** The areas Stage 3 of `performance-pass` covers, each a folder under `benches/scaling/`. */
export const AREAS = [
  "compiler", "values", "arrays", "strings", "regex", "json", "markup", "formats", "templates", "numbers", "time",
  "database", "queue", "cache", "scheduler", "request", "connections", "traffic", "http-client", "lsp", "fmt", "app",
] as const;

/** What a `serve` ladder's size counts; the first is the default. */
export const SERVE_SIZES = ["requests", "headers", "header-bytes", "body-bytes", "query", "connections"] as const;

/** How many requests one load sends when the size shapes each request rather than counting them. */
export const SHAPED_REQUESTS = 1024;

/** The value of every header a `size headers` request adds. */
export const HEADER_VALUE = "a".repeat(32);

/** How many requests each open connection sends in a `size connections` load. */
export const PER_CONNECTION = 32;

/** A ladder's `// scaling:` lines. `size` is a `serve` ladder's alone, and empty for every other kind. */
export interface Ladder {
  kind: string;
  start: number;
  max: number;
  expect: string;
  proposal: boolean;
  size: string;
}

/** The ladder `source` declares, or what is wrong with its `// scaling:` lines. */
export function ladderOf(source: string): Ladder | string {
  const seen: Record<string, string> = {};
  for (const m of source.matchAll(/^\/\/\s*scaling:\s*([a-z]+)(?:[ \t]+([^\s]+))?[ \t]*\r?$/gm)) seen[m[1]!] = m[2] ?? "";
  const unknown = Object.keys(seen).find((k) => !["kind", "start", "max", "expect", "proposal", "size"].includes(k));
  if (unknown) return `\`// scaling: ${unknown}\` is not a ladder line`;
  const kind = seen.kind || "run";
  if (!(KINDS as readonly string[]).includes(kind)) return `\`// scaling: kind ${kind}\` is not one of ${KINDS.join(", ")}`;
  if ("size" in seen && kind !== "serve") return "`// scaling: size` belongs to a `serve` ladder alone";
  const size = kind === "serve" ? seen.size || SERVE_SIZES[0] : "";
  if (kind === "serve" && !(SERVE_SIZES as readonly string[]).includes(size)) return `\`// scaling: size ${size}\` is not one of ${SERVE_SIZES.join(", ")}`;
  if (!/^[0-9][0-9_]*$/.test(seen.start ?? "") || !/^[0-9][0-9_]*$/.test(seen.max ?? "")) return "it needs `// scaling: start N` and `// scaling: max N`";
  const expect = seen.expect ?? "";
  if (expect === "quadratic") return "`// scaling: expect quadratic` is never accepted";
  if (!(expect in EXPECT)) return `\`// scaling: expect\` must be one of ${Object.keys(EXPECT).join(", ")}`;
  return { kind, start: number(seen.start!), max: number(seen.max!), expect, proposal: "proposal" in seen, size };
}

/** The sizes a ladder runs: `start`, doubling, up to `max`. */
export function ladderSizes(start: number, max: number): number[] {
  const sizes: number[] = [];
  for (let s = start; s >= 1 && s <= max; s *= 2) sizes.push(s);
  return sizes;
}

/** The bounds a ladder that declares `expect` is judged against. */
export const boundsOf = (expect: string): Bounds => ({ count: EXPECT[expect]!, clock: EXPECT[expect]! + CLOCK_BOUND - COUNT_BOUND });

interface Options {
  nvs: string;
  reps: number;
  wslNvs: string;
  callgrind: boolean;
  scratch: string;
}

/**
 * `nvs.toml` as it reads from `copyDir`: every quoted path that resolves, from `fromDir`, into `copied`
 * names the same file in the copy under `root`, and every other path that exists names the original.
 * A path in a configuration resolves from the folder the file is in, so a copy moved elsewhere needs
 * this to keep its capabilities, its app and its includes.
 */
export function rebased(text: string, fromDir: string, copyDir: string, copied: string, root: string): string {
  return text.replace(/"([^"\\\r\n]+)"/g, (whole, path: string) => {
    if (isAbsolute(path) || /^[a-z][a-z0-9+.-]*:/i.test(path)) return whole;
    const target = resolve(fromDir, path);
    const inside = relative(copied, target);
    const moved = !inside.startsWith("..") && !isAbsolute(inside);
    if (!moved && !existsSync(target)) return whole;
    const there = moved ? join(root, relative(ROOT, target)) : target;
    return `"${relative(copyDir, there).replace(/\\/g, "/") || "."}"`;
  });
}

/**
 * A copy of the bench's whole folder under `root`, so its `.in`, `.nvsr` and the files it loads come
 * with it, and an `nvs.toml` beside the copy: the folder's own, or the repository's, rebased. Returns the
 * copy's repo-relative path.
 */
function copyBench(bench: string, root: string): string {
  const dir = abs(dirname(bench));
  const copyDir = join(root, dirname(bench));
  cpSync(dir, copyDir, { recursive: true });
  const own = existsSync(join(dir, "nvs.toml"));
  const from = own ? dir : ROOT;
  writeFileSync(join(copyDir, "nvs.toml"), rebased(readFileSync(join(from, "nvs.toml"), "utf8"), from, copyDir, dir, root));
  return rel(join(root, bench));
}

/** The fastest of `reps` plain runs of `copy`, in nanoseconds. */
async function timeRun(nvs: string, copy: string, bench: string, reps: number): Promise<number> {
  let best = Infinity;
  for (let i = 0; i < reps; i++) {
    const out = await spawnProof([nvs, "run", copy], copy, TIMEOUT_MS, { unlogged: true });
    if (out.code !== 0) throw new PerfError(`${bench} exited ${out.code} at a batch: ${out.stderr.trim().split(/\r?\n/)[0] ?? ""}`);
    best = Math.min(best, out.ms * 1e6);
  }
  return best;
}

/** Callgrind's instruction total for one `nvs <command> <file>`. */
async function instructions(opts: Options, command: string, file: string, bench: string): Promise<number> {
  const argv = ["valgrind", "--tool=callgrind", "--callgrind-out-file=/dev/null", process.platform === "win32" ? opts.wslNvs : opts.nvs, command, file];
  const out = await spawnProof(process.platform === "win32" ? ["wsl.exe", "--", ...argv] : argv, file, TIMEOUT_MS * 4, { unlogged: true });
  const m = COLLECTED_RE.exec(out.stderr);
  if (!m) throw new PerfError(`callgrind printed no instruction total for ${bench}: ${out.stderr.trim().split(/\r?\n/).at(-1) ?? ""}`);
  return Number(m[1]);
}

/** How one kind of program is measured at one size, once its copy holds that size. */
interface Measure {
  /** The counts the agreement test and the bounds judge. */
  keys: readonly string[];
  /** Whether the copy's closing literal is rewritten to the size. A kind that is not rewritten is
   * handed the size instead. */
  rewrites: boolean;
  take(copy: string, bench: string, opts: Options, size: number): Promise<{ counts: Record<string, number>; ns: number }>;
  /** The fastest of `--reps` timings alone, for the clock's second run. */
  clock(copy: string, bench: string, opts: Options, size: number): Promise<number>;
  /** Callgrind's instructions for the same work. */
  callgrind(copy: string, bench: string, opts: Options, size: number): Promise<number>;
  /** A failure the kind judges on its own over the whole ramp, beside the counts and the clock. */
  judge?(batches: Batch[]): string | null;
}

/** A program that runs: `nvs run --count` and the clock of `nvs run`. */
const RUN: Measure = {
  keys: COUNTS,
  rewrites: true,
  take: async (copy, bench, opts) => ({ counts: await countProgram(opts.nvs, copy), ns: await timeRun(opts.nvs, copy, bench, opts.reps) }),
  clock: (copy, bench, opts) => timeRun(opts.nvs, copy, bench, opts.reps),
  callgrind: (copy, bench, opts) => instructions(opts, "run", copy, bench),
};

/** The program a `compile` ladder prints, written beside its copy. Returns its repo-relative path. */
async function generate(copy: string, bench: string, opts: Options): Promise<string> {
  const out = await spawnProof([opts.nvs, "run", copy], copy, TIMEOUT_MS, { unlogged: true });
  if (out.code !== 0) throw new PerfError(`${bench} exited ${out.code} while printing its program: ${out.stderr.trim().split(/\r?\n/)[0] ?? ""}`);
  const program = copy.replace(/\.nvs$/, ".printed.nvs");
  writeFileSync(abs(program), out.stdout);
  return program;
}

/** The fastest of `reps` runs of `nvs check` on `program`, in nanoseconds. */
async function timeCheck(nvs: string, program: string, bench: string, reps: number): Promise<number> {
  let best = Infinity;
  for (let i = 0; i < reps; i++) {
    const out = await spawnProof([nvs, "check", program], program, TIMEOUT_MS, { unlogged: true });
    if (out.code !== 0) throw new PerfError(`the program ${bench} printed does not check: ${out.stderr.trim().split(/\r?\n/)[0] ?? ""}`);
    best = Math.min(best, out.ms * 1e6);
  }
  return best;
}

/** A program the ladder prints: the `compile:` line of `nvs check --count`, with `ir` from `nvs run
 * --count`, and the clock of `nvs check`. */
const COMPILE: Measure = {
  keys: [...COMPILE_COUNTS, "total"],
  rewrites: true,
  take: async (copy, bench, opts) => {
    const program = await generate(copy, bench, opts);
    const counts: Record<string, number> = {};
    for (const command of ["check", "run"]) {
      const out = await spawnProof([opts.nvs, command, "--count", program], program, TIMEOUT_MS, { unlogged: true });
      const line = compileCounts(out.stderr);
      if (out.code !== 0 || line === null) throw new PerfError(`\`nvs ${command} --count\` on the program ${bench} printed exited ${out.code}: ${out.stderr.trim().split(/\r?\n/)[0] ?? ""}`);
      Object.assign(counts, line);
    }
    const missing = COMPILE_COUNTS.find((k) => !(k in counts));
    if (missing) throw new PerfError(`the \`compile:\` line has no \`${missing}\` -- is ${opts.nvs} built from this tree?`);
    counts.total = COMPILE_COUNTS.reduce((sum, k) => sum + counts[k]!, 0);
    return { counts, ns: await timeCheck(opts.nvs, program, bench, opts.reps) };
  },
  clock: async (copy, bench, opts) => timeCheck(opts.nvs, await generate(copy, bench, opts), bench, opts.reps),
  callgrind: async (copy, bench, opts) => instructions(opts, "check", await generate(copy, bench, opts), bench),
};

/** The fastest of `reps` runs of `nvs fmt --check` on `program`, in nanoseconds. It exits 1 for a file
 * that would change, so only a refusal fails. */
async function timeFmt(nvs: string, program: string, bench: string, reps: number): Promise<number> {
  let best = Infinity;
  for (let i = 0; i < reps; i++) {
    const out = await spawnProof([nvs, "fmt", "--check", program], program, TIMEOUT_MS, { unlogged: true });
    if (out.code > 1 || out.stderr.includes("does not parse")) throw new PerfError(`\`nvs fmt\` refused the program ${bench} printed: ${out.stderr.trim().split(/\r?\n/)[0] ?? ""}`);
    best = Math.min(best, out.ms * 1e6);
  }
  return best;
}

/** A file the ladder prints, formatted: the clock of `nvs fmt --check`, and no counts. */
const FMT: Measure = {
  keys: [],
  rewrites: true,
  take: async (copy, bench, opts) => ({ counts: {}, ns: await timeFmt(opts.nvs, await generate(copy, bench, opts), bench, opts.reps) }),
  clock: async (copy, bench, opts) => timeFmt(opts.nvs, await generate(copy, bench, opts), bench, opts.reps),
  callgrind: async (copy, bench, opts) => instructions(opts, "fmt", await generate(copy, bench, opts), bench),
};

/** The URI the `lsp` ladder's document is opened at. No file is behind it and no root is named, so the
 * server indexes the open document alone and the session is the same under WSL as on Windows. */
export const LSP_URI = "file:///scaling/ladder.nvs";
/** The cursor a printed document marks, as an `.lspt` case marks it. */
const CURSOR = "<|>";

/** One message, framed the way `nvs lsp` reads it from stdin. */
export const lspFrame = (message: object): string => {
  const body = JSON.stringify({ jsonrpc: "2.0", ...message });
  return `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`;
};

/**
 * The session an `lsp` ladder runs on the document it printed, in the order it is sent: the
 * handshake, open, one edit that appends a comment line, then completion, hover and references at the
 * `<|>` the document marks, then shutdown and exit. Requests carry ids 1 to 5 in that order. Returns
 * null when the document marks no cursor.
 */
export function lspScript(printed: string): object[] | null {
  const at = printed.indexOf(CURSOR);
  if (at < 0) return null;
  const text = printed.slice(0, at) + printed.slice(at + CURSOR.length);
  const before = text.slice(0, at).split("\n");
  const position = { line: before.length - 1, character: before.at(-1)!.length };
  const doc = { textDocument: { uri: LSP_URI }, position };
  return [
    { id: 1, method: "initialize", params: { processId: null, rootUri: null, capabilities: {} } },
    { method: "initialized", params: {} },
    { method: "textDocument/didOpen", params: { textDocument: { uri: LSP_URI, languageId: "nvs", version: 1, text } } },
    { method: "textDocument/didChange", params: { textDocument: { uri: LSP_URI, version: 2 }, contentChanges: [{ text: `${text}// edited\n` }] } },
    { id: 2, method: "textDocument/completion", params: doc },
    { id: 3, method: "textDocument/hover", params: doc },
    { id: 4, method: "textDocument/references", params: { ...doc, context: { includeDeclaration: true } } },
    { id: 5, method: "shutdown", params: null },
    { method: "exit", params: null },
  ];
}

/** The messages `nvs lsp` has written so far, as they arrive on its stdout. */
class LspReader {
  private buffer = Buffer.alloc(0);
  private readonly got: Record<string, unknown>[] = [];
  private wake: (() => void) | null = null;
  private ended = false;

  constructor(stdout: ReadableStream<Uint8Array>) {
    void (async () => {
      for await (const chunk of stdout) {
        this.buffer = Buffer.concat([this.buffer, chunk]);
        for (;;) {
          const end = this.buffer.indexOf("\r\n\r\n");
          if (end < 0) break;
          const length = Number(/Content-Length:\s*(\d+)/i.exec(this.buffer.subarray(0, end).toString())?.[1]);
          if (this.buffer.length < end + 4 + length) break;
          this.got.push(JSON.parse(this.buffer.subarray(end + 4, end + 4 + length).toString("utf8")));
          this.buffer = this.buffer.subarray(end + 4 + length);
        }
        this.wake?.();
      }
      this.ended = true;
      this.wake?.();
    })();
  }

  /** The first message `match` accepts, removed from what has arrived. */
  async next(match: (m: Record<string, unknown>) => boolean, what: string): Promise<Record<string, unknown>> {
    for (;;) {
      const at = this.got.findIndex(match);
      if (at >= 0) return this.got.splice(at, 1)[0]!;
      if (this.ended) throw new PerfError(`\`nvs lsp\` exited before it sent ${what}`);
      await new Promise<void>((wake) => (this.wake = wake));
    }
  }
}

/** One `nvs lsp` session over the script, timed from the open to the answer to references, in
 * nanoseconds. Each step waits for what it causes: the open for its diagnostics, a request for its
 * response. The edit's analysis runs when the completion request arrives, so it is inside the clock. */
async function lspSession(nvs: string, script: object[], bench: string): Promise<number> {
  const child = Bun.spawn([nvs, "lsp"], { cwd: ROOT, stdin: "pipe", stdout: "pipe", stderr: "ignore", env: { LLVM_PROFILE_FILE: DISCARD_PROFILE, ...process.env } });
  const reader = new LspReader(child.stdout);
  const timer = setTimeout(() => child.kill(), TIMEOUT_MS);
  const send = (m: object) => {
    child.stdin.write(lspFrame(m));
    child.stdin.flush();
  };
  const answer = async (m: object) => {
    const id = (m as { id: number }).id;
    send(m);
    const got = await reader.next((r) => r.id === id && !("method" in r), `an answer to ${(m as { method: string }).method}`);
    if (got.error) throw new PerfError(`\`nvs lsp\` answered ${(m as { method: string }).method} on the document ${bench} printed with an error: ${JSON.stringify(got.error)}`);
  };
  try {
    const [init, initialized, open, edit, ...rest] = script;
    const [completion, hover, references, shutdown, exit] = rest;
    await answer(init!);
    send(initialized!);
    const started = performance.now();
    send(open!);
    await reader.next((m) => m.method === "textDocument/publishDiagnostics" && (m.params as { uri: string }).uri === LSP_URI, "diagnostics for the opened document");
    send(edit!);
    for (const request of [completion!, hover!, references!]) await answer(request);
    const ns = (performance.now() - started) * 1e6;
    await answer(shutdown!);
    send(exit!);
    child.stdin.end();
    await child.exited;
    return ns;
  } finally {
    clearTimeout(timer);
    child.kill();
  }
}

/** The fastest of `reps` sessions on the document the ladder prints. */
async function timeLsp(nvs: string, printed: string, bench: string, reps: number): Promise<number> {
  const script = lspScript(readFileSync(abs(printed), "utf8"));
  if (script === null) throw new PerfError(`the document ${bench} printed marks no \`${CURSOR}\` cursor`);
  let best = Infinity;
  for (let i = 0; i < reps; i++) best = Math.min(best, await lspSession(nvs, script, bench));
  return best;
}

/** Callgrind's instruction total for the whole session, its messages piped in from a file beside the copy. */
async function lspInstructions(opts: Options, printed: string, bench: string): Promise<number> {
  const script = lspScript(readFileSync(abs(printed), "utf8"));
  if (script === null) throw new PerfError(`the document ${bench} printed marks no \`${CURSOR}\` cursor`);
  const feed = printed.replace(/\.nvs$/, ".lspin");
  writeFileSync(abs(feed), script.map(lspFrame).join(""));
  const nvs = process.platform === "win32" ? opts.wslNvs : opts.nvs;
  const shell = ["sh", "-c", `valgrind --tool=callgrind --callgrind-out-file=/dev/null '${nvs}' lsp < '${feed}'`];
  const out = await spawnProof(process.platform === "win32" ? ["wsl.exe", "--", ...shell] : shell, printed, TIMEOUT_MS * 4, { unlogged: true });
  const m = COLLECTED_RE.exec(out.stderr);
  if (!m) throw new PerfError(`callgrind printed no instruction total for ${bench}: ${out.stderr.trim().split(/\r?\n/).at(-1) ?? ""}`);
  return Number(m[1]);
}

/** A document the ladder prints, opened, edited and asked about in `nvs lsp`: the clock alone, as `fmt`. */
const LSP: Measure = {
  keys: [],
  rewrites: true,
  take: async (copy, bench, opts) => ({ counts: {}, ns: await timeLsp(opts.nvs, await generate(copy, bench, opts), bench, opts.reps) }),
  clock: async (copy, bench, opts) => timeLsp(opts.nvs, await generate(copy, bench, opts), bench, opts.reps),
  callgrind: async (copy, bench, opts) => lspInstructions(opts, await generate(copy, bench, opts), bench),
};

/** The load a `serve` ladder whose size counts `what` sends at size `n`: the path and shape of every
 * request, how many requests, and over how many keep-alive connections. */
export function serveShape(what: string, n: number): { path: string; shape: HttpShape; requests: number; concurrency: number } {
  const one = { path: "/", shape: {}, requests: SHAPED_REQUESTS, concurrency: 1 };
  switch (what) {
    case "requests":
      return { ...one, requests: n };
    case "headers":
      return { ...one, shape: { headers: Array.from({ length: n }, (_, i): [string, string] => [`x-field-${i}`, HEADER_VALUE]) } };
    case "header-bytes":
      return { ...one, shape: { headers: [["x-field", "a".repeat(n)]] } };
    case "body-bytes":
      return { ...one, shape: { body: Buffer.alloc(n, "a") } };
    case "query":
      return { ...one, path: `/?${Array.from({ length: n - 1 }, (_, i) => `p${i}=value&`).join("")}q=1` };
    case "connections":
      return { ...one, requests: n * PER_CONNECTION, concurrency: n };
    default:
      throw new PerfError(`\`size ${what}\` has no load`);
  }
}

/** The load `serveShape` gives for this size, sent to an `nvs serve` of the copy booted for this size
 * alone and stopped after it: the fastest of `reps` loads in nanoseconds, and the lowest peak memory the
 * server process reached across them, in the unit the OS reports. */
async function serveLoad(copy: string, bench: string, opts: Options, what: string, size: number): Promise<{ ns: number; peak: number }> {
  const load = serveShape(what, size);
  let ns = Infinity;
  let peak = Infinity;
  const why = (e: unknown) => (e instanceof Error ? e.message : String(e)).trim().split(/\r?\n/)[0] ?? "";
  for (let i = 0; i < opts.reps; i++) {
    const port = await freePort();
    const argv = [opts.nvs, "serve", abs(copy), "--listen", `127.0.0.1:${port}`];
    const server = await Server.start(argv, port, dirname(abs(copy)), { LLVM_PROFILE_FILE: DISCARD_PROFILE }).catch((e: unknown) => {
      throw new PerfError(`\`nvs serve\` did not start on ${bench}: ${why(e)}`);
    });
    try {
      ns = Math.min(ns, (await hammer(() => HttpConn.open(port, load.path, load.shape), load.requests, load.concurrency)).elapsed * 1e9);
    } catch (e) {
      throw new PerfError(`\`nvs serve\` failed a request at size ${size} (${what}) of ${bench}: ${why(e)}`);
    } finally {
      await server.stop();
    }
    const p = server.peakMemory();
    if (p === null) throw new PerfError(`the OS reported no peak memory for the server ${bench} ran`);
    peak = Math.min(peak, p);
  }
  return { ns, peak };
}

/** Why a `serve` ramp's peak memory grew past `PEAK_SLACK`, or null when it stayed flat. */
export function peakGrows(batches: Batch[]): string | null {
  const peaks = batches.map((b) => b.counts.peak!);
  const low = Math.min(...peaks);
  const last = peaks.at(-1)!;
  if (last <= low * (1 + PEAK_SLACK)) return null;
  return `peak memory grows with requests served: ${fixed((last / low - 1) * 100, 0)}% above its lowest at ${batches.at(-1)!.size} requests`;
}

/** The program every request runs, under the load its size `what` shapes: the clock of the whole load,
 * and the server's peak memory, which `peakGrows` holds flat when the size counts requests served. */
const serve = (what: string): Measure => ({
  keys: [],
  rewrites: false,
  take: async (copy, bench, opts, size) => {
    const { ns, peak } = await serveLoad(copy, bench, opts, what, size);
    return { counts: { peak }, ns };
  },
  clock: async (copy, bench, opts, size) => (await serveLoad(copy, bench, opts, what, size)).ns,
  callgrind: (_copy, bench) => Promise.reject(new PerfError(`the clock's slope of ${bench} could not be read, and a \`serve\` ladder is not run under callgrind`)),
  ...(what === "requests" ? { judge: peakGrows } : {}),
});

/** `body`, run while an `nvs serve` of the `fetch` ladder's `<name>/server.nvs` listens on a free port
 * and `FETCH_URL_ENV` names it to every program `body` starts. */
async function withServer<T>(bench: string, opts: Options, body: () => Promise<T>): Promise<T> {
  const fixture = abs(bench.replace(/\.nvs$/, "/server.nvs"));
  const port = await freePort();
  const argv = [opts.nvs, "serve", fixture, "--listen", `127.0.0.1:${port}`];
  const server = await Server.start(argv, port, dirname(fixture), { LLVM_PROFILE_FILE: DISCARD_PROFILE }).catch((e: unknown) => {
    throw new PerfError(`\`nvs serve\` did not start on the server ${bench} fetches from: ${(e instanceof Error ? e.message : String(e)).trim().split(/\r?\n/)[0] ?? ""}`);
  });
  const before = process.env[FETCH_URL_ENV];
  process.env[FETCH_URL_ENV] = `http://127.0.0.1:${port}`;
  try {
    return await body();
  } finally {
    if (before === undefined) delete process.env[FETCH_URL_ENV];
    else process.env[FETCH_URL_ENV] = before;
    await server.stop();
  }
}

/** A program that downloads from the server beside it: `run`'s counts and clock, taken while that server listens. */
const FETCH: Measure = {
  keys: COUNTS,
  rewrites: true,
  take: (copy, bench, opts, size) => withServer(bench, opts, () => RUN.take(copy, bench, opts, size)),
  clock: (copy, bench, opts, size) => withServer(bench, opts, () => RUN.clock(copy, bench, opts, size)),
  callgrind: (_copy, bench) => Promise.reject(new PerfError(`${bench} could not be judged, and a \`fetch\` ladder is not run under callgrind`)),
};

const MEASURES: Record<string, Measure> = { run: RUN, compile: COMPILE, lsp: LSP, fmt: FMT, fetch: FETCH };

/** One bench, ramped and judged. */
async function rampOne(bench: string, opts: Options): Promise<Judged> {
  const judged: Judged = { bench, verdict: "skipped", sizes: [], slopes: {}, clock: null, notes: [] };
  const source = read(bench);
  const skip = skipReason(source);
  if (skip) return { ...judged, notes: [skip] };
  const m = ITER_RE.exec(source);
  if (!m) return { ...judged, notes: ["declares no `// bench: iterations N`"] };
  const iterations = number(m[1]!);
  const sizes = batchSizes(iterations);
  if (sizes.length < MIN_BATCHES) return { ...judged, notes: [`\`iterations ${iterations}\` is too few to ramp`] };
  if (withBatch(source, iterations, sizes[0]!) === null) return { ...judged, notes: ["its closing `echo Bench::run(...)` does not pass `iterations` as one literal"] };
  return rampAt(judged, source, iterations, sizes, BENCH_BOUNDS, RUN, opts);
}

/** One ladder, ramped over its sizes and judged against what it declares. */
async function ladderOne(bench: string, opts: Options): Promise<Judged> {
  const judged: Judged = { bench, verdict: "invalid", sizes: [], slopes: {}, clock: null, notes: [] };
  const source = read(bench);
  const ladder = ladderOf(source);
  if (typeof ladder === "string") return { ...judged, notes: [ladder] };
  const skip = skipReason(source);
  if (skip) return { ...judged, verdict: "skipped", notes: [skip] };
  const measure = ladder.kind === "serve" ? serve(ladder.size) : MEASURES[ladder.kind]!;
  const sizes = ladderSizes(ladder.start, ladder.max);
  if (sizes.length < MIN_BATCHES) return { ...judged, notes: [`\`start ${ladder.start}\` to \`max ${ladder.max}\` is fewer than ${MIN_BATCHES} doublings`] };
  if (measure.rewrites && withBatch(source, ladder.start, ladder.start) === null) return { ...judged, notes: ["its closing `echo Bench::run(...)` does not pass `start` as one literal"] };
  return proposed(await rampAt(judged, source, ladder.start, sizes, boundsOf(ladder.expect), measure, opts), ladder);
}

/** A ladder marked `proposal` that grows waits for the user's decision, and does not fail. */
export function proposed(j: Judged, ladder: Ladder): Judged {
  if (j.verdict !== "grows" || !ladder.proposal) return j;
  return { ...j, verdict: "proposal", notes: [...j.notes, "marked `proposal`: it waits for the user's decision"] };
}

/** Runs `source` at each of `sizes`, its closing literal `literal` rewritten, until the counts agree,
 * and judges the ramp against `bounds`. */
async function rampAt(judged: Judged, source: string, literal: number, sizes: number[], bounds: Bounds, measure: Measure, opts: Options): Promise<Judged> {
  const bench = judged.bench;
  const copy = copyBench(bench, join(opts.scratch, bench.replace(/[\\/]/g, "~")));
  const batches: Batch[] = [];
  const write = (size: number) => writeFileSync(abs(copy), measure.rewrites ? withBatch(source, literal, size)! : source);
  for (const size of sizes) {
    progress(`scaling: ${bench} at ${size}`);
    write(size);
    batches.push({ size, ...(await measure.take(copy, bench, opts, size)) });
    if (measure.keys.length && countsAgree(batches, measure.keys)) break;
  }
  judged.sizes = batches.map((b) => b.size);
  const { slopes, over } = judgeCounts(batches, measure.keys, bounds.count);
  const own = measure.judge?.(batches);
  if (own) over.push(own);
  judged.slopes = slopes;
  judged.clock = clockSlope(batches);
  let clockGrows: string | null = null;
  if (judged.clock !== null && judged.clock > bounds.clock) {
    // The clock never fails on one run: the same batches are timed again, and only agreement counts.
    const again: Batch[] = [];
    for (const b of batches) {
      write(b.size);
      again.push({ ...b, ns: await measure.clock(copy, bench, opts, b.size) });
    }
    const second = clockSlope(again);
    if (second !== null && second > bounds.clock) clockGrows = `the clock grows with slope ${fixed(judged.clock, 2)}, and ${fixed(second, 2)} on a second run`;
    else judged.notes.push(`the clock's slope ${fixed(judged.clock, 2)} did not repeat on a second run`);
  }
  if (over.length) return { ...judged, verdict: "grows", notes: [...judged.notes, ...over, ...(clockGrows ? [clockGrows] : [])] };
  // A kind with no counts has only the clock, so the clock decides.
  if (!measure.keys.length && clockGrows) return { ...judged, verdict: "grows", notes: [...judged.notes, clockGrows] };
  if (!measure.keys.length && judged.clock !== null) return { ...judged, verdict: "flat" };
  // Counts that settled within their bound are not failed by the clock alone, since the second run
  // shares the first one's machine load: callgrind settles it.
  if (measure.keys.length && countsAgree(batches, measure.keys) && !clockGrows) return { ...judged, verdict: "flat" };
  if (clockGrows) judged.notes.push(`${clockGrows}, and the counts do not`);
  if (!opts.callgrind) return { ...judged, verdict: "unclear", notes: [...judged.notes, clockGrows ? "--no-callgrind is set" : "the counts did not agree by the ceiling, and --no-callgrind is set"] };
  const ir: Batch[] = [];
  for (const b of batches) {
    progress(`scaling: ${bench} at ${b.size} under callgrind`);
    write(b.size);
    ir.push({ ...b, counts: { instructions: await measure.callgrind(copy, bench, opts, b.size) } });
  }
  const cg = judgeCounts(ir, ["instructions"], bounds.count);
  judged.slopes.instructions = cg.slopes.instructions ?? null;
  if (cg.over.length) return { ...judged, verdict: "grows", notes: [...judged.notes, ...cg.over] };
  if (agrees(increments(ir, (b) => b.counts.instructions!))) return { ...judged, verdict: "flat" };
  return { ...judged, verdict: "unclear", notes: [...judged.notes, "neither the counts nor callgrind's instructions agreed by the ceiling"] };
}

/** Every program under `tree`, repo-relative and sorted. The calibration is not one, and neither is a
 * file in the folder a program `<name>.nvs` keeps beside it as `<name>/`. */
function programs(tree: string): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    if (!existsSync(abs(dir))) return;
    for (const name of readdirSync(abs(dir)).sort()) {
      const path = `${dir}/${name}`;
      if (statSync(abs(path)).isDirectory()) {
        if (name !== CALIBRATION && !existsSync(abs(`${path}.nvs`))) walk(path);
      } else if (name.endsWith(".nvs")) out.push(path);
    }
  };
  walk(tree);
  return out;
}

/** The areas with no ladder, and with `reviewed`, also the areas `REVIEW` has no `## <area>` section for. */
export function missingAreas(ladders: string[], review: string | null): { noLadder: string[]; noReview: string[] } {
  const noLadder = AREAS.filter((a) => !ladders.some((l) => l.startsWith(`${LADDERS}/${a}/`)));
  const headings = new Set((review ?? "").split(/\r?\n/).filter((l) => l.startsWith("## ")).map((l) => l.slice(3).replace(/`/g, "").trim()));
  return { noLadder, noReview: review === null ? [] : AREAS.filter((a) => !headings.has(a)) };
}

function areas(reviewed: boolean): number {
  const review = reviewed ? (existsSync(abs(REVIEW)) ? readFileSync(abs(REVIEW), "utf8") : "") : null;
  const { noLadder, noReview } = missingAreas(programs(LADDERS), review);
  if (noLadder.length) console.log(`nv scaling: ${noLadder.length} of ${AREAS.length} areas have no ladder under ${LADDERS}/: ${noLadder.join(", ")}`);
  if (noReview.length) console.log(`nv scaling: ${noReview.length} of ${AREAS.length} areas have no section in ${REVIEW}: ${noReview.join(", ")}`);
  if (noLadder.length || noReview.length) return 1;
  console.log(reviewed ? "every area has a ladder and a review section" : "every area has a ladder");
  return 0;
}

const showSlope = (s: number | null | undefined) => (s === null || s === undefined ? "-" : fixed(s, 2));

function line(j: Judged): string {
  const at = j.sizes.length ? `${j.sizes[0]}..${j.sizes.at(-1)}` : "";
  const counts = !j.sizes.length ? "" : Object.keys(j.slopes).map((k) => `${k} ${showSlope(j.slopes[k])}`).join("  ") + `  clock ${showSlope(j.clock)}`;
  const head = `  ${j.verdict.padEnd(8)} ${j.bench.replace(/^benches\/[^/]+\//, "")}  ${at}  ${counts}`.trimEnd();
  return [head, ...j.notes.map((n) => `            ${n}`)].join("\n");
}

function arg(args: string[], name: string): string | undefined {
  const at = args.indexOf(name);
  if (at < 0) return undefined;
  const value = args[at + 1];
  args.splice(at, 2);
  return value;
}

export async function run(argv: string[]): Promise<number> {
  const args = [...argv];
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(`${summary}

    (no mode)             ramp every ladder under ${LADDERS}/ over its sizes
    --iterations          ramp every bench under ${BENCHES}/ over small batches instead
    --check               print one line when nothing grows past its bound, for an acceptance check
    --areas               fail while an area has no ladder; with --reviewed, also no section in ${REVIEW}
    <filter>...           only the programs whose path contains one of these
    --nvs <path>          the binary to run, instead of the release build
    --reps <n>            plain runs per batch, the fastest is the clock (default 2)
    --jobs <n>            benches ramped at once (default 1); more is faster and makes the clock noisier
    --no-callgrind        report a bench that does not settle as unclear, without callgrind
    --all                 print the flat benches too, not only the ones that grow or are unclear
    --wsl-nvs <path>      the Linux binary callgrind runs in WSL (default ${DEFAULT_WSL_NVS})

The ramp, the agreement test, the ceiling and the bounds are in tools/nv/cmd/scaling.ts.`);
    return 0;
  }
  const flag = (name: string) => {
    const at = args.indexOf(name);
    if (at >= 0) args.splice(at, 1);
    return at >= 0;
  };
  if (flag("--areas")) {
    const reviewed = flag("--reviewed");
    if (args.length) {
      console.error(`nv scaling: --areas takes no other argument, not ${args[0]}`);
      return 2;
    }
    return areas(reviewed);
  }
  const iterations = flag("--iterations");
  const check = flag("--check");
  const named = arg(args, "--nvs");
  const reps = Number(arg(args, "--reps") ?? 2);
  const jobs = Math.max(1, Number(arg(args, "--jobs") ?? 1));
  const wslNvs = arg(args, "--wsl-nvs") ?? DEFAULT_WSL_NVS;
  const callgrind = !args.includes("--no-callgrind");
  const all = args.includes("--all");
  const filters = args.filter((a) => a !== "--no-callgrind" && a !== "--all");
  const unknown = filters.find((a) => a.startsWith("-"));
  if (unknown) {
    console.error(`nv scaling: unknown argument ${unknown}`);
    return 2;
  }
  const todo = programs(iterations ? BENCHES : LADDERS).filter((b) => filters.length === 0 || filters.some((f) => b.includes(f)));
  if (todo.length === 0) {
    console.log(`nv scaling: no ${iterations ? "bench" : "ladder"} matches`);
    if (check && filters.length === 0) console.log(iterations ? PASSED_ITERATIONS : PASSED_LADDERS);
    return 0;
  }
  let nvs = named;
  if (!nvs) {
    const built = await releaseBinary();
    if (typeof built === "string") {
      console.error(`nv scaling: ${built}`);
      return 1;
    }
    nvs = built.path;
  }
  const scratch = join(ROOT, ".agent-tmp", `scaling-${process.pid}`);
  const opts: Options = { nvs, reps, wslNvs, callgrind, scratch };
  console.log(iterations ? `nv scaling: ${todo.length} benches, batches from ${START} doubling to at most ${CEILING}, ${rel(nvs)}` : `nv scaling: ${todo.length} ladders, ${rel(nvs)}`);
  const results: Judged[] = [];
  let next = 0;
  const worker = async () => {
    while (next < todo.length) {
      const bench = todo[next++]!;
      let j: Judged;
      try {
        j = await (iterations ? rampOne : ladderOne)(bench, opts);
      } catch (e) {
        if (!(e instanceof PerfError)) throw e;
        // A bench that fails at a small batch is one to look at; a ladder that fails is broken.
        j = { bench, verdict: iterations ? "unclear" : "invalid", sizes: [], slopes: {}, clock: null, notes: [e.message] };
      }
      results.push(j);
      if (all || j.verdict !== "flat") console.log(line(j));
    }
  };
  try {
    await Promise.all(Array.from({ length: Math.min(jobs, todo.length) }, worker));
  } finally {
    if (existsSync(scratch)) rmSync(scratch, { recursive: true, force: true });
  }
  const tally = (v: Verdict) => results.filter((r) => r.verdict === v).length;
  const extra = iterations ? "" : `, ${tally("proposal")} proposal, ${tally("invalid")} invalid`;
  console.log(`nv scaling: ${tally("flat")} flat, ${tally("grows")} grow, ${tally("unclear")} unclear, ${tally("skipped")} skipped${extra}`);
  if (tally("grows") + tally("invalid") > 0) return 1;
  if (check) console.log(iterations ? PASSED_ITERATIONS : PASSED_LADDERS);
  return 0;
}
