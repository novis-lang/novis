// `bun nv against`: whether a change to the checker moved what any program lowers to, what any program
// is told, or how long checking takes, measured against a recorded base build.
//
//     bun nv against --record          copy target/release/nvs and its commit as the base, and dump it
//     bun nv against                   compare the current target/release/nvs with the base
//     bun nv against --rounds 9        more timing pairs per program
//     bun nv against --corpus <dir>    only the programs under <dir>; may be given more than once
//
// The corpus is every `.nvs` file under `benches/members/` and `docs/examples/`. Each program gets one
// `nvs run --dump-ir` and one `nvs check`, run from the program's own folder. `--record` copies the
// release binary and the commit `HEAD` names to `.agent-tmp/against/base/`, dumps every program twice,
// and fails when the two dumps of one program differ: a dump that is not deterministic would hide a real
// difference later. It stores each program's IR as a hash and its diagnostics as text, beside the count
// of `assertTrusted` and `assertTrustedBytes` calls in the programs a reader copies.
//
// The compare dumps every program once with the current binary and prints:
//
// - every program whose IR differs from the base's, with its first differing line. When the hash
//   differs, the base binary dumps the program's current source again. If that dump is the current
//   binary's, only the source changed since the base, and the program is listed apart as edited: it
//   does not fail the compare;
// - every program whose diagnostics differ, with both sets;
// - the `assertTrusted` count, then and now;
// - the median ratio of `nvs check`'s time, current over base, over the 40 largest programs. Each program
//   runs as `--rounds` pairs, one run of each binary, and the order inside a pair alternates, so a burst
//   of load lands on both alike. It prints each program's median ratio and both medians.
//
// It exits 1 when an IR differs or the ratio is over 1.03, the budget `taint-fail-closed` set. A
// diagnostic difference is reported and does not fail: it is the list of programs a change made stop
// compiling, and each one is read by hand. Build the binary first with `bun nv relink --build`.
//
// What it spends: each binary keeps its own data folder under `.agent-tmp/against/`, and the base keeps
// a copy of the release binary, about 100 MB. The dumps run on most of the cores, about five minutes for
// a record and three for a compare; the timing runs one program at a time, about a minute at 5 rounds.
// Deleting `.agent-tmp/against/` drops the base.

import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { basename, dirname, join } from "node:path";
import { ROOT, rel } from "../lib/paths.ts";
import { pool, run as runProc } from "../lib/proc.ts";
import { releaseCli } from "../lib/relink.ts";
import { median } from "./start-cost.ts";

export const summary =
  "record a base nvs, then compare a build's IR, diagnostics and check time against it: nv against [--record] [--rounds N] [--corpus DIR]";

/** The folders whose programs make the corpus. */
export const CORPUS = ["benches/members", "docs/examples"];
/** The folders whose programs readers copy, where `assertTrusted` calls are counted. */
export const COPIED = ["docs/examples", "tests/hostile", "benches/members", "website/snippets"];
/** How many of the largest programs the timing runs. */
export const LARGEST = 40;
/** The most the median check-time ratio may be. */
export const BUDGET = 1.03;

const EXE = process.platform === "win32" ? "nvs.exe" : "nvs";

export interface Args {
  record: boolean;
  rounds: number;
  corpus: string[];
}

/** The command's arguments, or the message that says what is wrong with them. */
export function parse(args: string[]): Args | string {
  let record = false;
  let rounds = 5;
  const corpus: string[] = [];
  for (let i = 0; i < args.length; i++) {
    const arg = args[i]!;
    if (arg === "--record") record = true;
    else if (arg === "--rounds") {
      const next = args[++i];
      if (next === undefined) return "--rounds needs a number";
      const n = Number(next);
      if (!Number.isInteger(n) || n < 1) return `--rounds needs a whole number of 1 or more, not ${next}`;
      rounds = n;
    } else if (arg === "--corpus") {
      const next = args[++i];
      if (next === undefined) return "--corpus needs a folder";
      corpus.push(next);
    } else return `unknown argument ${arg}`;
  }
  return { record, rounds, corpus: corpus.length === 0 ? CORPUS : corpus };
}

/** Every `.nvs` file under `roots`, as paths from `ROOT` with `/`, sorted. */
export function programs(roots: string[]): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (entry.name.endsWith(".nvs")) out.push(rel(path).replaceAll("\\", "/"));
    }
  };
  for (const root of roots) walk(join(ROOT, root));
  return out.sort();
}

/** The `n` largest of `files` by size, largest first, ties by path. */
export function largest(files: string[], n: number): string[] {
  const size = new Map(files.map((f) => [f, statSync(join(ROOT, f)).size]));
  return [...files].sort((a, b) => size.get(b)! - size.get(a)! || a.localeCompare(b)).slice(0, n);
}

/** How many times `Core\Taint::assertTrusted` or `::assertTrustedBytes` is called in the files under
 *  `roots`. */
export function assertTrustedCalls(roots: string[]): number {
  let count = 0;
  const walk = (dir: string) => {
    if (!existsSync(dir)) return;
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) walk(path);
      else count += (readFileSync(path, "utf8").match(/assertTrusted(?:Bytes)?\s*\(/g) ?? []).length;
    }
  };
  for (const root of roots) walk(join(ROOT, root));
  return count;
}

/** What one binary says about one program: its IR and its diagnostics, each with the exit status. */
export interface Output {
  ir: string;
  diag: string;
}

/** One binary, with the data folder it alone uses. */
export interface Binary {
  exe: string;
  data: string;
}

const text = (code: number, stdout: string, stderr: string) =>
  `exit ${code}\n${stdout.replace(/\r\n/g, "\n")}${stderr === "" ? "" : `--- stderr\n${stderr.replace(/\r\n/g, "\n")}`}`;

async function invoke(bin: Binary, program: string, argv: string[]) {
  const started = performance.now();
  const result = await runProc([bin.exe, "--data", bin.data, ...argv, basename(program)], {
    cwd: join(ROOT, dirname(program)),
    timeoutMs: 60_000,
  });
  if (result.timedOut) throw new Error(`\`nvs ${argv.join(" ")} ${program}\` did not finish${result.hung ? `: ${result.hung}` : ""}`);
  return { ms: performance.now() - started, out: text(result.code, result.stdout, result.stderr) };
}

/** One program's IR dump and check, by `bin`. */
export async function output(bin: Binary, program: string): Promise<Output> {
  const ir = (await invoke(bin, program, ["run", "--dump-ir"])).out;
  const diag = (await invoke(bin, program, ["check"])).out;
  return { ir, diag };
}

/** Every program's output by `bin`, on most of the cores, with a progress line on standard error. */
export async function outputs(bin: Binary, files: string[], label: string): Promise<Map<string, Output>> {
  const out = new Map<string, Output>();
  let done = 0;
  await pool(files, Math.max(1, availableParallelism() - 2), async (file) => {
    out.set(file, await output(bin, file));
    if (++done % 250 === 0) process.stderr.write(`against: ${label} ${done}/${files.length}\n`);
  });
  return out;
}

/** The record `--record` writes: the base's commit, its `assertTrusted` count, and per program the IR's
 *  hash and the diagnostics. */
export interface Base {
  commit: string;
  assertTrusted: number;
  programs: Record<string, { ir: string; diag: string }>;
}

export const hash = (s: string): string => Bun.hash(s).toString(16);

/** The work folder's layout. */
export function layout(work: string) {
  return {
    base: { exe: join(work, "base", EXE), data: join(work, "base", "data") },
    current: { exe: releaseCli(), data: join(work, "current-data") },
    record: join(work, "base", "base.json"),
  };
}

/** The programs whose two dumps differ, each with its first differing line. */
export function unsteady(first: Map<string, Output>, second: Map<string, Output>): string[] {
  const out: string[] = [];
  for (const [file, a] of first) {
    const b = second.get(file)!;
    if (a.ir !== b.ir) out.push(`${file}: IR ${firstDifference(a.ir, b.ir)}`);
    if (a.diag !== b.diag) out.push(`${file}: diagnostics ${firstDifference(a.diag, b.diag)}`);
  }
  return out;
}

/** Where two texts first differ, as `line N: <a> | <b>`. */
export function firstDifference(a: string, b: string): string {
  const x = a.split("\n");
  const y = b.split("\n");
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    if (x[i] !== y[i]) return `line ${i + 1}: ${x[i] ?? "(end)"} | ${y[i] ?? "(end)"}`;
  }
  return "no line differs";
}

/** `--record`: copy the release binary, dump every program twice, and write the record. */
export async function record(work: string, roots: string[]): Promise<number> {
  const paths = layout(work);
  const release = releaseCli();
  if (!existsSync(release)) {
    console.error(`nv against: ${rel(release)} does not exist; run \`bun nv relink --build\` first`);
    return 1;
  }
  rmSync(join(work, "base"), { recursive: true, force: true });
  mkdirSync(paths.base.data, { recursive: true });
  copyFileSync(release, paths.base.exe);
  const head = await runProc(["git", "rev-parse", "HEAD"]);
  const commit = head.stdout.trim();
  const files = programs(roots);
  const first = await outputs(paths.base, files, "first dump");
  const second = await outputs(paths.base, files, "second dump");
  const differ = unsteady(first, second);
  if (differ.length > 0) {
    console.error(`nv against: ${differ.length} output(s) differ between two dumps of one build, so no base is recorded:`);
    for (const line of differ.slice(0, 20)) console.error(`  ${line}`);
    rmSync(join(work, "base"), { recursive: true, force: true });
    return 1;
  }
  const base: Base = { commit, assertTrusted: assertTrustedCalls(COPIED), programs: {} };
  for (const [file, o] of first) base.programs[file] = { ir: hash(o.ir), diag: o.diag };
  writeFileSync(paths.record, JSON.stringify(base));
  console.log(`against: base recorded from ${commit.slice(0, 10)}: ${files.length} programs, ${base.assertTrusted} assertTrusted calls`);
  return 0;
}

/** One program's timing: its median ratio, current over base, and each binary's median. */
export interface Timing {
  file: string;
  ratios: number[];
  baseMs: number;
  currentMs: number;
}

/** `rounds` pairs of `nvs check` per program, the order inside a pair alternating. */
export async function timings(base: Binary, current: Binary, files: string[], rounds: number): Promise<Timing[]> {
  const out: Timing[] = [];
  for (const file of files) {
    // One unmeasured run of each, so neither pays for a cold data folder.
    await invoke(base, file, ["check"]);
    await invoke(current, file, ["check"]);
    const b: number[] = [];
    const c: number[] = [];
    const ratios: number[] = [];
    for (let i = 0; i < rounds; i++) {
      const order = i % 2 === 0 ? [base, current] : [current, base];
      const ms = [];
      for (const bin of order) ms.push((await invoke(bin, file, ["check"])).ms);
      const [bm, cm] = i % 2 === 0 ? [ms[0]!, ms[1]!] : [ms[1]!, ms[0]!];
      b.push(bm);
      c.push(cm);
      ratios.push(cm / bm);
    }
    out.push({ file, ratios, baseMs: median(b), currentMs: median(c) });
  }
  return out;
}

/** What a compare found. */
export interface Compare {
  irDiffers: string[];
  /** Programs whose source changed since the base, and that both binaries lower alike. */
  edited: string[];
  diagDiffers: { file: string; base: string; current: string }[];
  added: string[];
  assertTrusted: { base: number; current: number };
  ratio: number;
  timings: Timing[];
}

/** Compare the current release binary with the recorded base. */
export async function compare(work: string, roots: string[], rounds: number): Promise<Compare | string> {
  const paths = layout(work);
  if (!existsSync(paths.record)) return `no base is recorded; run \`bun nv against --record\` first`;
  if (!existsSync(paths.current.exe)) return `${rel(paths.current.exe)} does not exist; run \`bun nv relink --build\` first`;
  const base = JSON.parse(readFileSync(paths.record, "utf8")) as Base;
  mkdirSync(paths.current.data, { recursive: true });
  const files = programs(roots);
  const known = files.filter((f) => base.programs[f] !== undefined);
  const now = await outputs(paths.current, known, "dump");
  const irDiffers: string[] = [];
  const edited: string[] = [];
  const diagDiffers: Compare["diagDiffers"] = [];
  for (const file of known) {
    const was = base.programs[file]!;
    const is = now.get(file)!;
    if (hash(is.ir) !== was.ir) {
      const old = await output(paths.base, file);
      if (old.ir === is.ir) edited.push(file);
      else irDiffers.push(`${file}: ${firstDifference(old.ir, is.ir)}`);
    }
    if (is.diag !== was.diag) diagDiffers.push({ file, base: was.diag, current: is.diag });
  }
  const timed = await timings(paths.base, paths.current, largest(known, LARGEST), rounds);
  return {
    irDiffers,
    edited,
    diagDiffers,
    added: files.filter((f) => base.programs[f] === undefined),
    assertTrusted: { base: base.assertTrusted, current: assertTrustedCalls(COPIED) },
    ratio: median(timed.flatMap((t) => t.ratios)),
    timings: timed,
  };
}

/** The compare as the lines it prints. */
export function report(c: Compare, rounds: number): string[] {
  const lines: string[] = [];
  lines.push(`IR: ${c.irDiffers.length === 0 ? "every program lowers as the base does" : `${c.irDiffers.length} program(s) differ`}`);
  for (const line of c.irDiffers) lines.push(`  ${line}`);
  if (c.edited.length > 0) {
    lines.push(`edited since the base, and both binaries lower the new source alike: ${c.edited.length} program(s)`);
    for (const file of c.edited) lines.push(`  ${file}`);
  }
  lines.push(`diagnostics: ${c.diagDiffers.length === 0 ? "every program is told what the base told it" : `${c.diagDiffers.length} program(s) differ`}`);
  for (const d of c.diagDiffers) {
    lines.push(`  ${d.file}`, "    base:", ...d.base.trimEnd().split("\n").map((l) => `      ${l}`));
    lines.push("    now:", ...d.current.trimEnd().split("\n").map((l) => `      ${l}`));
  }
  if (c.added.length > 0) lines.push(`not in the base, so not compared: ${c.added.length} program(s)`, ...c.added.map((f) => `  ${f}`));
  lines.push(`assertTrusted calls: ${c.assertTrusted.base} in the base, ${c.assertTrusted.current} now`);
  lines.push(`check time, current over base, ${rounds} pairs on each of the ${c.timings.length} largest programs:`);
  for (const t of c.timings) {
    lines.push(
      `  ${median(t.ratios).toFixed(3)}  ${t.baseMs.toFixed(1).padStart(7)} ms -> ${t.currentMs.toFixed(1).padStart(7)} ms  ${t.file}`,
    );
  }
  lines.push(`median ratio: ${c.ratio.toFixed(3)} (budget ${BUDGET})`);
  return lines;
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const parsed = parse(args);
  if (typeof parsed === "string") {
    console.error(`nv against: ${parsed}`);
    return 2;
  }
  const work = join(ROOT, ".agent-tmp", "against");
  if (parsed.record) return record(work, parsed.corpus);
  const c = await compare(work, parsed.corpus, parsed.rounds);
  if (typeof c === "string") {
    console.error(`nv against: ${c}`);
    return 1;
  }
  for (const line of report(c, parsed.rounds)) console.log(line);
  return c.irDiffers.length > 0 || c.ratio > BUDGET ? 1 : 0;
}
