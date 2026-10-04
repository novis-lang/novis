// `bun nv scaling`: runs a program at growing sizes and reports how fast its cost grows. The rule it
// enforces is the user's: linear is correct and a little steeper is fine, and quadratic or steeper is a
// defect. `benches/scaling/README.md` is what a ladder is; this doc is the one home of the ramp, the
// agreement test, the ceiling and the bounds.
//
//     bun nv scaling --iterations                every bench under benches/members/
//     bun nv scaling --iterations core/Str       only the benches whose path contains `core/Str`
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
// is reported. Above `CLOCK_BOUND` it fails only when a second run of the same batches agrees, because
// the clock is the second signal and never the first. A clock increment at or under zero leaves the
// clock's slope unread.
//
// A bench that reaches the ceiling and has not agreed runs the same batches again under
// `valgrind --tool=callgrind`, in WSL on Windows, with the Linux binary `--wsl-nvs` names. Callgrind's
// instruction count is nearly the same on every run, so it is judged as one more count. A bench still
// unclear after that is reported under *unclear* and is not judged.
//
// The counts are the same on every machine, so a bench gets the same verdict everywhere. The clock is
// taken on the release binary unless `--nvs` names another, as `--record-perf` takes it.

import { cpSync, existsSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { abs, rel, ROOT } from "../lib/paths.ts";
import { progress } from "../lib/progress.ts";
import { fixed } from "../lib/py.ts";
import { countProgram, COUNTS, PerfError } from "../proofs/perf.ts";
import { read } from "../proofs/roster.ts";
import { releaseBinary, skipReason, spawnProof } from "../proofs/run.ts";

export const summary = "how fast a program's cost grows with its size: nv scaling --iterations [path-filter...]";

const BENCHES = "benches/members";
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
export function countsAgree(batches: Batch[]): boolean {
  return COUNTS.every((k) => agrees(increments(batches, (b) => b.counts[k]!)));
}

export type Verdict = "flat" | "grows" | "unclear" | "skipped";

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
export function judgeCounts(batches: Batch[], keys: readonly string[] = COUNTS): { slopes: Record<string, number | null>; over: string[] } {
  const slopes: Record<string, number | null> = {};
  const over: string[] = [];
  for (const k of keys) {
    const d = increments(batches, (b) => b.counts[k]!);
    const s = slopeOf(d);
    slopes[k] = s;
    if (s !== null && s > COUNT_BOUND) {
      over.push(`${k} grows with slope ${fixed(s, 2)}, ${fixed(d.at(-3)!, 3)} per op at ${batches.at(-3)!.size} and ${fixed(d.at(-1)!, 3)} at ${batches.at(-1)!.size}`);
    }
  }
  return { slopes, over };
}

const clockSlope = (batches: Batch[]) => slopeOf(increments(batches, (b) => b.ns));

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

/** Callgrind's instruction total for one run of `copy`. */
async function instructions(opts: Options, copy: string, bench: string): Promise<number> {
  const argv = ["valgrind", "--tool=callgrind", "--callgrind-out-file=/dev/null", process.platform === "win32" ? opts.wslNvs : opts.nvs, "run", copy];
  const out = await spawnProof(process.platform === "win32" ? ["wsl.exe", "--", ...argv] : argv, copy, TIMEOUT_MS * 4, { unlogged: true });
  const m = COLLECTED_RE.exec(out.stderr);
  if (!m) throw new PerfError(`callgrind printed no instruction total for ${bench}: ${out.stderr.trim().split(/\r?\n/).at(-1) ?? ""}`);
  return Number(m[1]);
}

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
  const copy = copyBench(bench, join(opts.scratch, bench.replace(/[\\/]/g, "~")));
  const batches: Batch[] = [];
  const write = (size: number) => writeFileSync(abs(copy), withBatch(source, iterations, size)!);
  for (const size of sizes) {
    progress(`scaling: ${bench} at ${size}`);
    write(size);
    const counts = await countProgram(opts.nvs, copy);
    batches.push({ size, counts, ns: await timeRun(opts.nvs, copy, bench, opts.reps) });
    if (countsAgree(batches)) break;
  }
  judged.sizes = batches.map((b) => b.size);
  const { slopes, over } = judgeCounts(batches);
  judged.slopes = slopes;
  judged.clock = clockSlope(batches);
  if (judged.clock !== null && judged.clock > CLOCK_BOUND) {
    // The clock never fails on one run: the same batches are timed again, and only agreement fails.
    const again: Batch[] = [];
    for (const b of batches) {
      write(b.size);
      again.push({ ...b, ns: await timeRun(opts.nvs, copy, bench, opts.reps) });
    }
    const second = clockSlope(again);
    if (second !== null && second > CLOCK_BOUND) over.push(`the clock grows with slope ${fixed(judged.clock, 2)}, and ${fixed(second, 2)} on a second run`);
    else judged.notes.push(`the clock's slope ${fixed(judged.clock, 2)} did not repeat on a second run`);
  }
  if (over.length) return { ...judged, verdict: "grows", notes: [...judged.notes, ...over] };
  if (countsAgree(batches)) return { ...judged, verdict: "flat" };
  if (!opts.callgrind) return { ...judged, verdict: "unclear", notes: [...judged.notes, "the counts did not agree by the ceiling, and --no-callgrind is set"] };
  const ir: Batch[] = [];
  for (const b of batches) {
    progress(`scaling: ${bench} at ${b.size} under callgrind`);
    write(b.size);
    ir.push({ ...b, counts: { instructions: await instructions(opts, copy, bench) } });
  }
  const cg = judgeCounts(ir, ["instructions"]);
  judged.slopes.instructions = cg.slopes.instructions ?? null;
  if (cg.over.length) return { ...judged, verdict: "grows", notes: [...judged.notes, ...cg.over] };
  if (agrees(increments(ir, (b) => b.counts.instructions!))) return { ...judged, verdict: "flat" };
  return { ...judged, verdict: "unclear", notes: [...judged.notes, "neither the counts nor callgrind's instructions agreed by the ceiling"] };
}

/** Every bench program under `benches/members/`, repo-relative and sorted. The calibration is not one,
 * and neither is a file in the folder a bench `<name>.nvs` keeps beside it as `<name>/`. */
function benches(): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    for (const name of readdirSync(abs(dir)).sort()) {
      const path = `${dir}/${name}`;
      if (statSync(abs(path)).isDirectory()) {
        if (name !== CALIBRATION && !existsSync(abs(`${path}.nvs`))) walk(path);
      } else if (name.endsWith(".nvs")) out.push(path);
    }
  };
  walk(BENCHES);
  return out;
}

const showSlope = (s: number | null | undefined) => (s === null || s === undefined ? "-" : fixed(s, 2));

function line(j: Judged): string {
  const at = j.sizes.length ? `${j.sizes[0]}..${j.sizes.at(-1)}` : "";
  const counts = j.verdict === "skipped" ? "" : [...COUNTS, ...("instructions" in j.slopes ? ["instructions"] : [])].map((k) => `${k} ${showSlope(j.slopes[k])}`).join("  ") + `  clock ${showSlope(j.clock)}`;
  const head = `  ${j.verdict.padEnd(7)} ${j.bench.slice(BENCHES.length + 1)}  ${at}  ${counts}`.trimEnd();
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
  if (args.some((a) => a === "-h" || a === "--help") || !args.includes("--iterations")) {
    console.log(`${summary}

    --iterations          ramp every bench under ${BENCHES}/ over small batches
    <filter>...           only the benches whose path contains one of these
    --nvs <path>          the binary to run, instead of the release build
    --reps <n>            plain runs per batch, the fastest is the clock (default 2)
    --jobs <n>            benches ramped at once (default 1); more is faster and makes the clock noisier
    --no-callgrind        report a bench that does not settle as unclear, without callgrind
    --all                 print the flat benches too, not only the ones that grow or are unclear
    --wsl-nvs <path>      the Linux binary callgrind runs in WSL (default ${DEFAULT_WSL_NVS})

The ramp, the agreement test, the ceiling and the bounds are in tools/nv/cmd/scaling.ts.`);
    return args.includes("--iterations") || args.some((a) => a === "-h" || a === "--help") ? 0 : 2;
  }
  args.splice(args.indexOf("--iterations"), 1);
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
  let nvs = named;
  if (!nvs) {
    const built = await releaseBinary();
    if (typeof built === "string") {
      console.error(`nv scaling: ${built}`);
      return 1;
    }
    nvs = built.path;
  }
  const todo = benches().filter((b) => filters.length === 0 || filters.some((f) => b.includes(f)));
  if (todo.length === 0) {
    console.log("nv scaling: no bench matches");
    return 0;
  }
  const scratch = join(ROOT, ".agent-tmp", `scaling-${process.pid}`);
  const opts: Options = { nvs, reps, wslNvs, callgrind, scratch };
  console.log(`nv scaling: ${todo.length} benches, batches from ${START} doubling to at most ${CEILING}, ${rel(nvs)}`);
  const results: Judged[] = [];
  let next = 0;
  const worker = async () => {
    while (next < todo.length) {
      const bench = todo[next++]!;
      let j: Judged;
      try {
        j = await rampOne(bench, opts);
      } catch (e) {
        if (!(e instanceof PerfError)) throw e;
        j = { bench, verdict: "unclear", sizes: [], slopes: {}, clock: null, notes: [e.message] };
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
  console.log(`nv scaling: ${tally("flat")} flat, ${tally("grows")} grow, ${tally("unclear")} unclear, ${tally("skipped")} skipped`);
  return tally("grows") > 0 ? 1 : 0;
}
