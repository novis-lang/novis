// `bun nv bench`: the userland benchmark suite, the CLI's warm-start figure, and `nvs serve` against PHP.
//
//     bun nv bench                        every case, every engine, 5 timed reps each
//     bun nv bench 05 regex               only cases whose name contains "05" or "regex"
//     bun nv bench --reps 9               more reps when a number looks noisy
//     bun nv bench --check                correctness only: do the engines agree? (no timing)
//     bun nv bench --engines nvs,php      narrow the roster; the default is all four
//     bun nv bench --php-mode default     PHP as installed, instead of with opcache+JIT
//     bun nv bench --json docs/perf/userland.ndjson   append one record per case
//     bun nv bench --warm-start --max-work-ms 6       the CLI's own start cost, budgeted
//     bun nv bench --serve-vs-fpm --record PATH       requests/sec against PHP, appended to PATH
//
// The cases live in `benches/userland/` as twins -- `NN-slug.nvs`, `.php`, `.py` and `.ts` -- and that
// directory's README owns what a case is and how to add one. This command owns only how they are measured.
//
// # The engine roster
//
// `ENGINES` is a list, not a pair, because `rule:tooling/bench-engine-list-is-data` says the comparison
// is data: the baseline subtraction, the table and the JSON record all iterate it, so another engine is
// an entry and a file suffix. A case is defined by its `.nvs` file. Every other engine attaches if its
// twin is on disk and is skipped with a warning if it is not, because a missing twin must never look
// like agreement. An engine whose executable is not installed fails every case it is asked to run.
//
// # What is measured
//
// Wall clock of the whole process, the minimum over N reps after one discarded warm-up. Every source of
// noise on a desktop adds time and none subtracts it, so the minimum is the estimate of the work, and
// the median is printed beside it so a busy machine shows as a gap between the two.
//
// `00-baseline` is a case like any other, and its time is also subtracted from every other case in the
// `work` columns, because every engine pays a fixed cost to start a process and read a file. `total`
// answers "what does this script cost at the command line" and `work` answers "how fast is the
// language"; `rule:tooling/bench-engine-list-is-data` says which claim quotes which. A ratio column is
// `<engine> / nvs` over `work`: above 1.0 means Novis is faster. It is a ratio of two figures taken on
// one machine seconds apart, the only comparison a wall clock supports; `rule:testing/perf-two-mechanisms`
// is why a history across machines is counted in instructions instead.
//
// Every engine's stdout must match byte for byte before any time is reported. A case that disagrees is
// `DIFF` and prints each output: a benchmark that quietly stopped doing the same work is worse than none.
//
// A binary under `target/debug/` is refused unless `--allow-debug` says the caller means it, since a
// debug build measures its assertions. Nothing here builds, and a warning says so when the binary is
// older than the newest file under `crates/`. PHP runs with opcache and the tracing JIT (`--php-mode
// jit`) because the project's target is stated against PHP with JIT; the mode is in every record.
//
// # The warm-start figure
//
// `--warm-start` measures one engine and one script, the baseline case, so it runs with no PHP, Python
// or Bun installed. The discarded warm-up is what makes it warm: the binary and the script are in the
// page cache, and the artifact that run published is what every timed rep loads, so the figure covers
// `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`'s read path and not a
// compile. A cache directory that is refused makes every rep compile, which is the shape a regression
// here takes.
//
// `--max-work-ms` budgets the total LESS `nvs --version`. Most of a process's wall clock is the
// operating system creating it, and a budget on the total budgets the machine. `--version` is the same
// binary, loader and page cache doing strictly less, so the difference is config load, compile and run.
// Load on a machine dilates a process multiplicatively rather than adding a constant, and a uniformly
// slow box has a tight median, so the level of that floor is the reading that tells a busy box from a
// quiet one: above `QUIET_FLOOR_MS` the leg prints "not measured" and passes. What this cannot see is a
// regression in the floor itself, which is why both figures are printed on every run.
//
// The configuration is named rather than left to `./nvs.toml`, which in this repository is a fixture the
// goals keep adding to. `SHIPPED_CONFIG` is what a user's start resolves, so a directive added to the
// product moves the figure and one added to this tree's housekeeping does not. The leg takes
// `WARM_START_REPS` because a minimum over a handful of samples is biased high, and that bias lands on
// a difference of two small numbers.
//
// # The serve-versus-FPM leg
//
// `--serve-vs-fpm` is M7's throughput figure: requests/sec for `nvs serve` against PHP with opcache.
// The generator is this file: a closed loop over keep-alive connections is a page of sockets, the same
// on every platform, and it reports no tail. The baseline is `php-cgi -b`, which is the SAPI FPM runs,
// speaking FastCGI the way nginx drives FPM, so PHP pays no HTTP parse and no proxy hop while `nvs serve`
// pays both: the comparison favours PHP. `php -S` is the fallback, named as the weaker peer in the
// record. With no PHP at all the leg measures `nvs serve` alone and records `baseline: null`, because a
// runner with no PHP is not a regression in the tree.
//
// Concurrency defaults to 1 because neither peer serves two requests at once here: `php-cgi -b` is one
// process, and `php -S` is serial. Above 1 the figure is Novis against a queue, and the value is in every
// record. Connections open before the clock starts, and both bodies must match before either number is
// reported. `--record PATH` appends the run, both peers and the caveats that applied, to a JSON array
// (`writeServeRecord`).
//
// # Records
//
// A record's `host` names the machine the way Bun does, a float with no fraction is written `12`, and
// the default `--python` is the `python` on `PATH`. An engine whose executable cannot start is a failed
// case. A record file may also hold older runs taken by a Python client that spent more of each request
// in the client, most visibly on the faster peer, so a requests/sec figure from this command does not
// continue those runs' series.
//
// # What `bun nv bench-load` borrows
//
// The load leg (`tools/nv/cmd/bench-load.ts`) imports the binary lookup, the stale-build warning, the
// free port, the server subprocess, the version probe and the record writer from here, and the proxied
// leg (`tools/nv/cmd/bench-proxied.ts`) imports the FastCGI client and the closed-loop generator as
// well, so each of them has one copy to keep correct.

import { createServer, connect, type AddressInfo, type Socket } from "node:net";
import { cpus, machine, release } from "node:os";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync, appendFileSync } from "node:fs";
import { basename, dirname, isAbsolute, join, normalize, relative, resolve, sep } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { ArgError, fixed, parseArgs, pyInt, pyRepr, splitlines } from "../lib/py.ts";
import { unrecorded } from "../lib/reads.ts";

export const summary = "the userland benchmark suite: nv bench [PATTERN...] | --check | --warm-start | --serve-vs-fpm";

const USAGE = [
  "usage: nv bench [-h] [--reps REPS] [--check] [--nvs NVS] [--php PHP]",
  "                [--python PYTHON] [--bun BUN] [--engines ENGINES]",
  "                [--php-mode {default,jit}] [--allow-debug] [--warm-start]",
  "                [--max-work-ms MS] [--json PATH] [--serve-vs-fpm]",
  "                [--record PATH] [--requests N] [--concurrency N]",
  "                [--serve-baseline {auto,fcgi,builtin,none}]",
  "                [patterns ...]",
].join("\n");

const WINDOWS = process.platform === "win32";
const CASE_DIR = join(ROOT, "benches", "userland");
const BASELINE = "00-baseline";
/** The configuration `--warm-start` measures against; the module doc says why it is named. */
const SHIPPED_CONFIG = join(ROOT, "crates", "nvs-config", "src", "default.toml");
/** The engine every ratio is taken against, and the one that defines a case. */
const PRIMARY = "nvs";
/** `--warm-start` abstains above this start floor; the module doc says why. */
const QUIET_FLOOR_MS = 5.75;
const SUITE_REPS = 5;
const WARM_START_REPS = 25;
const SERVE_DIR = join(ROOT, "benches", "serve");
/** The twin pair under `SERVE_DIR`. */
const SERVE_CASE = "hello";
/** Runs kept in `--record`'s artifact; `writeServeRecord` says why it is capped. */
const SERVE_HISTORY = 100;

/** PHP's invocation modes. The name is in every record, so a history never mixes the two. */
const PHP_MODES: Record<string, string[]> = {
  jit: ["-d", "opcache.enable_cli=1", "-d", "opcache.jit_buffer_size=64M", "-d", "opcache.jit=tracing"],
  default: [],
};

/** A message the command stops on, printed alone to stderr with exit status 1. */
export class Fail extends Error {}

// ---------------------------------------------------------------------------------------------------
// The command line.
// ---------------------------------------------------------------------------------------------------

interface Options {
  patterns: string[];
  reps: number | null;
  check: boolean;
  nvs: string | null;
  php: string;
  python: string;
  bun: string;
  engines: string;
  phpMode: string;
  allowDebug: boolean;
  warmStart: boolean;
  maxWorkMs: number | null;
  maxMs: number | null;
  json: string | null;
  serveVsFpm: boolean;
  record: string | null;
  requests: number;
  concurrency: number;
  serveBaseline: string;
}

const FLAGS = ["--check", "--allow-debug", "--warm-start", "--serve-vs-fpm"];
const VALUED = [
  "--reps", "--nvs", "--php", "--python", "--bun", "--engines", "--php-mode", "--max-work-ms", "--max-ms",
  "--json", "--record", "--requests", "--concurrency", "--serve-baseline",
];
const ORDER = [
  "--reps", "--check", "--nvs", "--php", "--python", "--bun", "--engines", "--php-mode", "--allow-debug",
  "--warm-start", "--max-work-ms", "--max-ms", "--json", "--serve-vs-fpm", "--record", "--requests",
  "--concurrency", "--serve-baseline",
];

/** Python's `float` over a command-line word, or null where it would raise. */
function pyFloat(word: string): number | null {
  const w = word.trim().replaceAll(/(?<=\d)_(?=\d)/g, "");
  if (/^[+-]?(inf|infinity)$/i.test(w)) return w.startsWith("-") ? -Infinity : Infinity;
  if (/^[+-]?nan$/i.test(w)) return NaN;
  return /^[+-]?(\d+\.?\d*|\.\d+)(e[+-]?\d+)?$/i.test(w) ? Number(w) : null;
}

/** The options argparse would have read, or null when `--help` asked for the help instead. */
function options(args: string[]): Options | null {
  const p = parseArgs(args, { flags: FLAGS, valued: VALUED, order: ORDER, positionals: true });
  if (p.flags.has("--help")) return null;
  const text = (o: string, dflt: string) => p.values.get(o) ?? dflt;
  const typed = (o: string, kind: "int" | "float", convert: (w: string) => number | null): number | null => {
    const word = p.values.get(o);
    if (word === undefined) return null;
    const n = convert(word);
    if (n === null) throw new ArgError(`argument ${o}: invalid ${kind} value: ${pyRepr(word)}`);
    return n;
  };
  const choice = (o: string, dflt: string, choices: string[]): string => {
    const word = p.values.get(o) ?? dflt;
    if (!choices.includes(word)) {
      throw new ArgError(`argument ${o}: invalid choice: ${pyRepr(word)} (choose from ${choices.map(pyRepr).join(", ")})`);
    }
    return word;
  };
  return {
    patterns: p.positionals,
    reps: typed("--reps", "int", pyInt),
    check: p.flags.has("--check"),
    nvs: p.values.get("--nvs") ?? null,
    php: text("--php", "php"),
    python: text("--python", Bun.which("python") ?? "python"),
    bun: text("--bun", "bun"),
    engines: text("--engines", "nvs,php,python,bun"),
    phpMode: choice("--php-mode", "jit", Object.keys(PHP_MODES).sort()),
    allowDebug: p.flags.has("--allow-debug"),
    warmStart: p.flags.has("--warm-start"),
    maxWorkMs: typed("--max-work-ms", "float", pyFloat),
    maxMs: typed("--max-ms", "float", pyFloat),
    json: p.values.get("--json") ?? null,
    serveVsFpm: p.flags.has("--serve-vs-fpm"),
    record: p.values.get("--record") ?? null,
    requests: typed("--requests", "int", pyInt) ?? 1000,
    concurrency: typed("--concurrency", "int", pyInt) ?? 1,
    serveBaseline: choice("--serve-baseline", "auto", ["auto", "fcgi", "builtin", "none"]),
  };
}

function help(): string {
  return [
    USAGE,
    "",
    "Run the userland benchmark suite across Novis, PHP, Python and Bun side by side.",
    "",
    "positional arguments:",
    "  patterns              substrings; only matching cases run",
    "",
    "options:",
    "  -h, --help            show this help message and exit",
    `  --reps REPS           timed reps per engine (default ${SUITE_REPS}, ${WARM_START_REPS} on --warm-start)`,
    "  --check               agreement only, no timing",
    "  --nvs NVS             path to the nvs binary (default target/release)",
    "  --php PHP             php executable (default `php`)",
    "  --python PYTHON       python executable (default: `python` on PATH)",
    "  --bun BUN             bun executable (default `bun`)",
    "  --engines ENGINES     comma-separated engine list, in table order; must include nvs (default all four)",
    "  --php-mode {default,jit}",
    "                        `jit` (default) runs PHP with opcache+tracing JIT; `default` runs it as installed",
    "  --allow-debug         permit a debug Novis binary",
    "  --warm-start          measure the CLI's start floor on the baseline case instead of running the suite",
    "  --max-work-ms MS      with --warm-start: exit non-zero if the total less `nvs --version` exceeds this budget",
    "  --json PATH           append one NDJSON record per case",
    "  --serve-vs-fpm        measure `nvs serve` requests/sec against PHP with opcache, instead of the suite",
    "  --record PATH         with --serve-vs-fpm: append the run to the JSON array at PATH",
    "  --requests N          with --serve-vs-fpm: requests per rep, per server (default 1000)",
    "  --concurrency N       with --serve-vs-fpm: keep-alive connections (default 1)",
    "  --serve-baseline {auto,fcgi,builtin,none}",
    "                        with --serve-vs-fpm: which PHP peer -- `auto` takes php-cgi over php -S",
  ].join("\n");
}

// ---------------------------------------------------------------------------------------------------
// Formatting the way the Python tool did.
// ---------------------------------------------------------------------------------------------------

/** `format(x, ",.Nf")`: fixed digits with a comma between thousands. */
export function grouped(x: number, digits: number): string {
  const s = fixed(x, digits);
  const [whole, frac] = s.split(".");
  const sign = whole!.startsWith("-") ? "-" : "";
  const digitsOnly = sign ? whole!.slice(1) : whole!;
  return sign + digitsOnly.replace(/\B(?=(\d{3})+(?!\d))/g, ",") + (frac === undefined ? "" : `.${frac}`);
}

const fmt = (value: number | null | undefined) => (value === null || value === undefined ? "-" : grouped(value, 1));
const ratioText = (value: number | null | undefined) => (value === null || value === undefined ? "-" : `${fixed(value, 2)}x`);
const listRepr = (items: string[]) => `[${items.map(pyRepr).join(", ")}]`;
export const round = (x: number, digits: number) => Number(fixed(x, digits));

/** `repr` of a byte string the way Python writes it. */
export function bytesRepr(b: Uint8Array): string {
  const has = (c: number) => b.includes(c);
  const q = has(0x27) && !has(0x22) ? '"' : "'";
  let body = "";
  for (const c of b) {
    if (c === 0x5c) body += "\\\\";
    else if (c === 0x09) body += "\\t";
    else if (c === 0x0a) body += "\\n";
    else if (c === 0x0d) body += "\\r";
    else if (c === q.charCodeAt(0)) body += `\\${q}`;
    else if (c >= 0x20 && c < 0x7f) body += String.fromCharCode(c);
    else body += `\\x${c.toString(16).padStart(2, "0")}`;
  }
  return `b${q}${body}${q}`;
}

/** `json.dumps` with its default separators, which the NDJSON history has always been written with. */
function pyDumps(v: unknown): string {
  if (Array.isArray(v)) return `[${v.map(pyDumps).join(", ")}]`;
  if (v !== null && typeof v === "object") {
    return `{${Object.entries(v).map(([k, x]) => `${JSON.stringify(k)}: ${pyDumps(x)}`).join(", ")}}`;
  }
  return JSON.stringify(v ?? null);
}

export function median(xs: number[]): number {
  const s = [...xs].sort((a, b) => a - b);
  const mid = s.length >> 1;
  return s.length % 2 ? s[mid]! : (s[mid - 1]! + s[mid]!) / 2;
}

/** A path as Python's `Path` prints it: this platform's separator, `.` segments dropped. */
const shownPath = (path: string) => normalize(path);

/** `binary` relative to the repository when it is inside it, as the records have always named it. */
export function repoRelative(binary: string): string {
  const full = resolve(binary);
  const r = relative(ROOT, full);
  return isAbsolute(binary) && !r.startsWith("..") && !isAbsolute(r) ? r : shownPath(binary);
}

export function stamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

export function host(): Record<string, unknown> {
  const all = cpus();
  return { platform: `${process.platform}-${release()}-${machine()}`, processor: all[0]?.model ?? "", cpus: all.length };
}

// ---------------------------------------------------------------------------------------------------
// Running a program, and timing it.
// ---------------------------------------------------------------------------------------------------

/** The first line a program prints for its version, or `unknown`. */
export function version(executable: string, argv: string[]): string {
  try {
    const out = Bun.spawnSync([executable, ...argv], { cwd: ROOT, stdout: "pipe", stderr: "pipe", timeout: 30_000 });
    const text = out.stdout.toString().trim();
    return text ? splitlines(text)[0]! : "unknown";
  } catch {
    return "unknown";
  }
}

/** Text mode's newline translation, so two engines on one platform compare as Python compared them. */
const textOf = (b: Uint8Array) => new TextDecoder().decode(b).replace(/\r\n?/g, "\n");

interface Once {
  elapsed: number;
  stdout: string;
  stderr: string;
  code: number;
}

/** One process, timed end to end. The wall clock includes start-up on purpose; the module doc says why. */
function runOnce(argv: string[]): Once {
  const start = performance.now();
  try {
    const p = Bun.spawnSync(argv, { cwd: ROOT, stdout: "pipe", stderr: "pipe" });
    const elapsed = performance.now() - start;
    return { elapsed, stdout: textOf(p.stdout), stderr: textOf(p.stderr), code: p.exitCode ?? 1 };
  } catch (e) {
    return { elapsed: performance.now() - start, stdout: "", stderr: `cannot run ${argv[0]}: ${(e as Error).message}`, code: 127 };
  }
}

interface Measured {
  failed: boolean;
  stdout: string;
  stderr: string;
  code?: number;
  min_ms?: number;
  median_ms?: number;
  samples_ms?: number[];
}

/** The measurement seam: one result per engine per case. Another measure is another key here and another column in `columnsFor`. */
function measure(argv: string[], reps: number): Measured {
  const first = runOnce(argv); // the warm-up, discarded
  if (first.code !== 0) return { failed: true, stdout: first.stdout, stderr: first.stderr, code: first.code };
  const samples: number[] = [];
  for (let i = 0; i < reps; i++) {
    const r = runOnce(argv);
    if (r.code !== 0 || r.stdout !== first.stdout) {
      return { failed: true, stdout: r.stdout, stderr: r.stderr || "output was not stable across reps", code: r.code };
    }
    samples.push(r.elapsed);
  }
  return { failed: false, stdout: first.stdout, stderr: first.stderr, min_ms: Math.min(...samples), median_ms: median(samples), samples_ms: samples };
}

export function findNvs(explicit: string | null, allowDebug: boolean): string {
  const binary = explicit ?? join(ROOT, "target", "release", WINDOWS ? "nvs.exe" : "nvs");
  if (!existsSync(binary)) {
    throw new Fail(`no Novis binary at ${shownPath(binary)}\n  build one with \`cargo build --release\`, or point --nvs at the one you mean`);
  }
  if (binary.split(/[\\/]/).includes("debug") && !allowDebug) {
    throw new Fail(
      `${shownPath(binary)} is a debug build, and a debug build measures the assertions rather than\n` +
        "  the language. Use target/release, or --allow-debug if you truly mean this one.",
    );
  }
  return binary;
}

/** Nothing here builds. If the binary predates the sources, the number is about old code. The warning
 * is no input to any verdict, so the files it looks at are not noted as reads of the run. */
export function warnIfStale(binary: string): void {
  let built = 0;
  let newest = -Infinity;
  try {
    unrecorded(() => {
      built = statSync(binary).mtimeMs;
      for (const p of new Bun.Glob("**/*.rs").scanSync({ cwd: join(ROOT, "crates") })) {
        newest = Math.max(newest, statSync(join(ROOT, "crates", p)).mtimeMs);
      }
    });
  } catch {
    return;
  }
  if (newest > built) {
    const age = (newest - built) / 3_600_000;
    console.error(
      `warning: ${basename(binary)} is ${fixed(age, 1)} h older than the newest file under crates/ --` +
        " these numbers are about the build on disk, not the tree",
    );
  }
}

// ---------------------------------------------------------------------------------------------------
// The warm-start figure.
// ---------------------------------------------------------------------------------------------------

/** What a second `nvs run` of the empty program costs, less what starting the binary at all costs. */
function warmStart(binary: string, reps: number, maxWorkMs: number | null): number {
  const source = join(CASE_DIR, `${BASELINE}.nvs`);
  if (!existsSync(source)) throw new Fail(`no ${source}: the warm-start figure is measured on the baseline case`);
  if (!existsSync(SHIPPED_CONFIG)) throw new Fail(`no ${SHIPPED_CONFIG}: the warm-start figure names the configuration it resolves`);

  const exe = resolve(binary);
  console.log(`nvs  ${shownPath(binary)}  (${version(exe, ["--version"])})`);
  const floor = measure([exe, "--version"], reps);
  const total = measure([exe, "run", "--config", SHIPPED_CONFIG, source], reps);
  for (const [what, result] of [["nvs --version", floor], ["nvs run", total]] as const) {
    if (result.failed) {
      console.log(`warm start FAILED: \`${what}\` exit ${result.code}`);
      console.error(result.stderr.trimEnd() || result.stdout.trimEnd());
      return 1;
    }
  }

  const work = total.min_ms! - floor.min_ms!;
  const label = relative(ROOT, source).split(sep).join("/");
  const config = relative(ROOT, SHIPPED_CONFIG).split(sep).join("/");
  console.log(`warm start  nvs run ${label}  (${reps} rep(s), one discarded warm-up)`);
  console.log(`  start floor ${fmt(floor.min_ms)} ms   nvs --version, the OS creating a process`);
  console.log(`  total       ${fmt(total.min_ms)} ms   median ${fmt(total.median_ms)} ms`);
  console.log(`  novis work  ${fmt(work)} ms   the total less that floor`);
  console.log(`  config      ${config}, so the figure moves with the binary rather than with this tree's own \`nvs.toml\``);
  console.log("  each rep is an `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` warm hit; the discarded warm-up published the artifact");
  if (maxWorkMs === null) return 0;
  if (floor.min_ms! > QUIET_FLOOR_MS) {
    console.log(
      `  not measured -- \`nvs --version\` alone costs ${fmt(floor.min_ms)} ms here, over the ${fmt(QUIET_FLOOR_MS)} ms ` +
        `a box this budget can be read on, so the ${fmt(work)} ms difference is a figure about the machine`,
    );
    return 0;
  }
  const within = work <= maxWorkMs;
  console.log(`  budget ${fmt(maxWorkMs)} ms on the work -- ${within ? "within" : "EXCEEDED"}`);
  return within ? 0 : 1;
}

// ---------------------------------------------------------------------------------------------------
// The suite.
// ---------------------------------------------------------------------------------------------------

interface Engine {
  key: string;
  /** What the table calls it: short, because every engine costs three columns. */
  label: string;
  suffix: string;
  argv: string[];
  versionArgv: string[];
}

interface Case {
  name: string;
  sources: Map<string, string>;
  description: string;
}

interface CaseRecord {
  case: string;
  description: string;
  engines: Map<string, Measured>;
  status: "ok" | "ERROR" | "DIFF";
  work?: Map<string, number>;
  ratio?: Map<string, number>;
}

function buildEngines(selected: string[], binary: string, o: Options): Engine[] {
  const available: Record<string, Engine> = {
    nvs: { key: "nvs", label: "nvs", suffix: ".nvs", argv: [resolve(binary), "run"], versionArgv: ["--version"] },
    php: { key: "php", label: "php", suffix: ".php", argv: [o.php, ...PHP_MODES[o.phpMode]!], versionArgv: ["-r", "echo PHP_VERSION;"] },
    python: { key: "python", label: "py", suffix: ".py", argv: [o.python], versionArgv: ["--version"] },
    bun: { key: "bun", label: "bun", suffix: ".ts", argv: [o.bun, "run"], versionArgv: ["--version"] },
  };
  const unknown = selected.filter((k) => !(k in available));
  if (unknown.length > 0) throw new Fail(`unknown engine(s) ${listRepr(unknown)}; known: ${Object.keys(available).join(", ")}`);
  if (!selected.includes(PRIMARY)) throw new Fail(`--engines must include ${PRIMARY}: every ratio is taken against it`);
  return selected.map((k) => available[k]!);
}

/** A case describes itself in its first `//` line, so there is no second manifest to update. */
function describe(path: string): string {
  for (const line of splitlines(readFileSync(path, "utf8")).slice(0, 6)) {
    const t = line.trim();
    if (t.startsWith("//")) return t.slice(2).trim();
  }
  return "";
}

/** Every `NN-slug.nvs`, in name order, with whichever twins are beside it. */
function discover(patterns: string[], engines: Engine[]): Case[] {
  const cases: Case[] = [];
  const names = [...new Bun.Glob("*.nvs").scanSync({ cwd: CASE_DIR })].sort((a, b) => (a.toLowerCase() < b.toLowerCase() ? -1 : 1));
  for (const name of names) {
    const stem = name.slice(0, -".nvs".length);
    const sources = new Map<string, string>();
    for (const engine of engines) {
      const source = join(CASE_DIR, stem + engine.suffix);
      if (existsSync(source)) sources.set(engine.key, source);
      else console.error(`warning: ${stem} has no ${engine.suffix} twin -- ${engine.key} skipped`);
    }
    if (sources.size < 2) {
      console.error(`warning: ${stem} has nothing to compare against -- skipped`);
      continue;
    }
    cases.push({ name: stem, sources, description: describe(join(CASE_DIR, name)) });
  }
  if (patterns.length > 0) {
    const kept = cases.filter((c) => c.name === BASELINE || patterns.some((p) => c.name.includes(p)));
    if (!kept.some((c) => c.name !== BASELINE)) throw new Fail(`no case matches ${listRepr(patterns)}; \`ls benches/userland\` lists them`);
    return kept;
  }
  return cases;
}

/** Runs every engine that has a twin for this case, and gates the timing on their agreement. */
function evaluate(c: Case, engines: Engine[], reps: number): CaseRecord {
  const results = new Map<string, Measured>();
  for (const engine of engines) {
    const source = c.sources.get(engine.key);
    if (source !== undefined) results.set(engine.key, measure([...engine.argv, source], reps));
  }
  const all = [...results.values()];
  const outputs = new Set(all.filter((r) => !r.failed).map((r) => r.stdout));
  const status = all.some((r) => r.failed) ? "ERROR" : outputs.size > 1 ? "DIFF" : "ok";
  return { case: c.name, description: c.description, engines: results, status };
}

/** `work` is `total` less the empty program, per engine; the ratio is over `work`. */
function subtractBaseline(records: CaseRecord[], engines: Engine[]): void {
  const baseline = records.find((r) => r.case === BASELINE && r.status === "ok");
  if (!baseline) return;
  for (const record of records) {
    if (record.status !== "ok") continue;
    const work = new Map<string, number>();
    for (const engine of engines) {
      const mine = record.engines.get(engine.key);
      const floor = baseline.engines.get(engine.key);
      if (mine && floor && !mine.failed && !floor.failed) work.set(engine.key, Math.max(0, mine.min_ms! - floor.min_ms!));
    }
    record.work = work;
    const primary = work.get(PRIMARY);
    if (primary) record.ratio = new Map([...work].filter(([k]) => k !== PRIMARY).map(([k, v]) => [k, v / primary]));
  }
}

/** Header, width, and how to read one row's value: total per engine, then work, then each ratio. */
function columnsFor(engines: Engine[]): [string, number, (r: CaseRecord) => string][] {
  return [
    ...engines.map((e): [string, number, (r: CaseRecord) => string] => [`${e.label} ms`, 10, (r) => fmt(r.engines.get(e.key)?.min_ms)]),
    ...engines.map((e): [string, number, (r: CaseRecord) => string] => [`${e.label} work`, 11, (r) => fmt(r.work?.get(e.key))]),
    ...engines
      .filter((e) => e.key !== PRIMARY)
      .map((e): [string, number, (r: CaseRecord) => string] => [`${e.label}/nvs`, 10, (r) => ratioText(r.ratio?.get(e.key))]),
  ];
}

function explain(record: CaseRecord): void {
  console.log(`${record.status}: ${record.case} -- ${record.description}`);
  for (const [key, result] of record.engines) {
    if (!result.failed) continue;
    console.log(`  ${key} exited ${result.code}`);
    for (const line of splitlines((result.stderr || "").trim()).slice(0, 12)) console.log(`    ${line}`);
  }
  if (record.status === "DIFF") {
    for (const [key, result] of record.engines) if (!result.failed) console.log(`  ${key} printed ${pyRepr(result.stdout)}`);
  }
}

function reportCheck(records: CaseRecord[]): void {
  for (const record of records) {
    if (record.status !== "ok") {
      explain(record);
      continue;
    }
    const agreed = [...record.engines.values()][0]!.stdout.trim();
    console.log(`ok    ${record.case}  [${[...record.engines.keys()].join("+")}]  ->  ${pyRepr(agreed)}`);
  }
}

function reportTable(records: CaseRecord[], engines: Engine[]): void {
  const columns = columnsFor(engines);
  const width = Math.max(...records.map((r) => r.case.length));
  const header = "case".padEnd(width) + columns.map(([h, w]) => h.padStart(w)).join("");
  console.log(header);
  console.log("-".repeat(header.length));
  for (const record of records) {
    if (record.status !== "ok") console.log(record.case.padEnd(width) + record.status.padStart(10));
    else console.log(record.case.padEnd(width) + columns.map(([, w, read]) => read(record).padStart(w)).join(""));
  }
  console.log("-".repeat(header.length));
  for (const engine of engines) {
    if (engine.key === PRIMARY) continue;
    const ratios = records.flatMap((r) => (r.status === "ok" && r.ratio?.has(engine.key) ? [[r.case, r.ratio.get(engine.key)!] as const] : []));
    if (ratios.length === 0) continue;
    const best = ratios.reduce((a, b) => (b[1] > a[1] ? b : a));
    const worst = ratios.reduce((a, b) => (b[1] < a[1] ? b : a));
    console.log(
      `vs ${engine.label.padEnd(4)} best ${best[0]} ${fixed(best[1], 2)}x |` +
        ` worst ${worst[0]} ${fixed(worst[1], 2)}x |` +
        ` median ${fixed(median(ratios.map(([, v]) => v)), 2)}x`,
    );
  }
  for (const record of records) {
    if (record.status === "ok") continue;
    console.log();
    explain(record);
  }
  const noisy = records.filter(
    (r) => r.status === "ok" && [...r.engines.values()].some((e) => !e.failed && e.median_ms! > 1.25 * e.min_ms!),
  );
  if (noisy.length > 0) {
    console.log(
      `note: min and median differ by more than 25% on ${noisy.map((r) => r.case).join(", ")} -- the machine was busy; re-run before quoting those.`,
    );
  }
}

/**
 * One NDJSON record per case. The `nvs_ms`/`php_ms`/`ratio` keys stay at the top level so the history
 * written before the suite grew a third engine still reads as one series; the rest is under `engines`.
 */
function writeJson(path: string, records: CaseRecord[], engines: Engine[], binary: string, mode: string, reps: number): void {
  mkdirSync(dirname(resolve(path)), { recursive: true });
  const date = stamp();
  const commit = version("git", ["rev-parse", "--short", "HEAD"]);
  const machineInfo = host();
  const versions = Object.fromEntries(engines.map((e) => [e.key, version(e.key === PRIMARY ? resolve(binary) : e.argv[0]!, e.versionArgv)]));
  let written = 0;
  let out = "";
  for (const record of records) {
    if (record.status !== "ok") continue;
    const perEngine: Record<string, { ms: number; work_ms: number; ratio: number | null }> = {};
    for (const [key, result] of record.engines) {
      if (result.failed) continue;
      const r = record.ratio?.get(key);
      perEngine[key] = { ms: round(result.min_ms!, 3), work_ms: round(record.work?.get(key) ?? 0, 3), ratio: r === undefined ? null : round(r, 4) };
    }
    out +=
      pyDumps({
        date,
        commit,
        case: record.case,
        nvs_ms: perEngine.nvs?.ms,
        php_ms: perEngine.php?.ms,
        nvs_work_ms: perEngine.nvs?.work_ms,
        php_work_ms: perEngine.php?.work_ms,
        ratio: perEngine.php?.ratio,
        engines: perEngine,
        engine_versions: versions,
        reps,
        php_mode: mode,
        nvs_binary: repoRelative(binary),
        host: machineInfo,
      }) + "\n";
    written++;
  }
  appendFileSync(path, out, "utf8");
  console.log(`appended ${written} record(s) to ${path}`);
}

function suite(binary: string, o: Options, reps: number): number {
  const selected = o.engines.split(",").map((n) => n.trim()).filter(Boolean);
  const engines = buildEngines(selected, binary, o);
  const cases = discover(o.patterns, engines);
  if (cases.length === 0) throw new Fail("no runnable case found under benches/userland");

  for (const engine of engines) {
    const target = engine.key === PRIMARY ? shownPath(binary) : engine.argv[0]!;
    const mode = engine.key === "php" ? ` [${o.phpMode}]` : "";
    console.log(`${engine.label.padEnd(4)} ${target}${mode}  (${version(engine.argv[0]!, engine.versionArgv)})`);
  }
  console.log(`${cases.length} case(s), ${reps} rep(s), min of reps; a ratio above 1.00 means Novis is faster`);
  console.log();

  const records: CaseRecord[] = [];
  for (const c of cases) {
    const record = evaluate(c, engines, reps);
    records.push(record);
    console.log(`  ${record.status.padEnd(5)} ${c.name}`);
  }
  subtractBaseline(records, engines);
  console.log();
  if (o.check) reportCheck(records);
  else reportTable(records, engines);
  if (o.json) writeJson(o.json, records, engines, binary, o.phpMode, reps);
  return records.every((r) => r.status === "ok") ? 0 : 1;
}

// ---------------------------------------------------------------------------------------------------
// The serve-versus-FPM leg. The module doc owns every decision -- which peer, why the generator is
// here, why concurrency is 1 -- and this half owns only how it is carried out.
// ---------------------------------------------------------------------------------------------------

/** A connection-level failure: the peer closed, reset or went silent. A request retries once on one. */
class Closed extends Error {}

const RECV_TIMEOUT_MS = 30_000;
const CONNECT_TIMEOUT_MS = 10_000;
const EMPTY: Buffer = Buffer.alloc(0);

/** One TCP connection, with what has arrived and not been read yet in `buf`. */
class Wire {
  buf: Buffer = EMPTY;
  private ended = false;
  private error: Error | null = null;
  private waiter: (() => void) | null = null;

  private constructor(private sock: Socket) {
    sock.setNoDelay(true);
    sock.setTimeout(RECV_TIMEOUT_MS);
    sock.on("data", (c: Buffer) => {
      this.buf = this.buf.length === 0 ? c : Buffer.concat([this.buf, c]);
      this.wake();
    });
    sock.on("end", () => {
      this.ended = true;
      this.wake();
    });
    sock.on("close", () => {
      this.ended = true;
      this.wake();
    });
    sock.on("error", (e) => {
      this.error = new Closed(e.message);
      this.wake();
    });
    sock.on("timeout", () => {
      this.error = new Closed("timed out");
      sock.destroy();
      this.wake();
    });
  }

  static open(port: number, timeoutMs: number): Promise<Wire> {
    return new Promise((ok, fail) => {
      const sock = connect({ host: "127.0.0.1", port });
      const timer = setTimeout(() => {
        sock.destroy();
        fail(new Closed(`timed out connecting to 127.0.0.1:${port}`));
      }, timeoutMs);
      const refused = (e: Error) => {
        clearTimeout(timer);
        fail(new Closed(e.message));
      };
      sock.once("error", refused);
      sock.once("connect", () => {
        clearTimeout(timer);
        sock.removeListener("error", refused);
        ok(new Wire(sock));
      });
    });
  }

  private wake(): void {
    const w = this.waiter;
    this.waiter = null;
    w?.();
  }

  /** Waits for the next thing the socket does: true when bytes arrived, false when the peer closed. */
  async more(): Promise<boolean> {
    const before = this.buf.length;
    if (this.error) throw this.error;
    if (this.ended) return false;
    await new Promise<void>((ok) => (this.waiter = ok));
    if (this.error) throw this.error;
    return this.buf.length > before || !this.ended;
  }

  write(b: Buffer): void {
    if (this.error) throw this.error;
    this.sock.write(b);
  }

  /** Exactly `n` bytes off the front of what arrived. */
  async take(n: number, closed: string): Promise<Buffer> {
    while (this.buf.length < n) if (!(await this.more())) throw new Closed(closed);
    const out = this.buf.subarray(0, n);
    this.buf = this.buf.subarray(n);
    return out;
  }

  close(): void {
    this.sock.destroy();
  }
}

interface Conn {
  reconnects: number;
  request(): Promise<Buffer>;
  close(): void;
}

/**
 * One keep-alive HTTP/1.1 connection, issuing one GET at a time. It reopens when the peer answers
 * `Connection: close`, which `php -S` does on every response, and `reconnects` counts it: a peer paying
 * a handshake per request and one that is not are two different measurements.
 */
class HttpConn implements Conn {
  reconnects = 0;
  private wire!: Wire;
  private readonly wireRequest: Buffer;

  private constructor(private port: number, path: string) {
    this.wireRequest = Buffer.from(`GET ${path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: */*\r\n\r\n`);
  }

  static async open(port: number, path = "/"): Promise<HttpConn> {
    const c = new HttpConn(port, path);
    c.wire = await Wire.open(port, CONNECT_TIMEOUT_MS);
    return c;
  }

  async request(): Promise<Buffer> {
    try {
      return await this.exchange();
    } catch (e) {
      if (!(e instanceof Closed)) throw e;
      this.reconnects++;
      this.wire.close();
      this.wire = await Wire.open(this.port, CONNECT_TIMEOUT_MS);
      return await this.exchange();
    }
  }

  private async exchange(): Promise<Buffer> {
    const w = this.wire;
    w.write(this.wireRequest);
    let at: number;
    while ((at = w.buf.indexOf("\r\n\r\n")) < 0) {
      if (!(await w.more())) throw new Closed("the server closed the connection mid-response");
    }
    const lines = w.buf.subarray(0, at).toString("latin1").toLowerCase().split("\r\n");
    w.buf = w.buf.subarray(at + 4);
    if (lines.some((l) => l.startsWith("transfer-encoding:"))) throw new Error("a chunked response is not measured here; the case sends a fixed body");
    const lengthLine = lines.find((l) => l.startsWith("content-length:"));
    const closing = lines.some((l) => l === "connection: close");
    let body: Buffer;
    if (lengthLine !== undefined) {
      body = await w.take(Number.parseInt(lengthLine.slice("content-length:".length).trim(), 10), "the server closed the connection mid-response");
    } else if (closing) {
      // HTTP/1.0 framing: the body ends when the peer closes, which is what `php -S` sends.
      while (await w.more());
      body = w.buf;
      w.buf = EMPTY;
    } else {
      throw new Error("a response framed by neither Content-Length nor a close is not read here");
    }
    if (closing) {
      // Reopened here rather than by failing the next request into a dead socket.
      this.reconnects++;
      w.close();
      this.wire = await Wire.open(this.port, CONNECT_TIMEOUT_MS);
    }
    return body;
  }

  close(): void {
    this.wire.close();
  }
}

// FastCGI 1.0: the record types and the two constants a responder request needs, which is the whole of
// the protocol used here and why there is a client below rather than a dependency.
const FCGI_BEGIN_REQUEST = 1;
const FCGI_END_REQUEST = 3;
const FCGI_PARAMS = 4;
const FCGI_STDIN = 5;
const FCGI_STDOUT = 6;
const FCGI_STDERR = 7;
const FCGI_RESPONDER = 1;
const FCGI_KEEP_CONN = 1;

function fcgiLen(n: number): Buffer {
  if (n < 128) return Buffer.from([n]);
  const b = Buffer.alloc(4);
  b.writeUInt32BE((n | 0x80000000) >>> 0);
  return b;
}

function fcgiPair(name: string, value: string): Buffer {
  const k = Buffer.from(name);
  const v = Buffer.from(value);
  return Buffer.concat([fcgiLen(k.length), fcgiLen(v.length), k, v]);
}

function fcgiRecord(kind: number, body: Buffer): Buffer {
  const head = Buffer.alloc(8);
  head.writeUInt8(1, 0);
  head.writeUInt8(kind, 1);
  head.writeUInt16BE(1, 2);
  head.writeUInt16BE(body.length, 4);
  return Buffer.concat([head, body]);
}

/**
 * One FastCGI connection to `php-cgi -b`, issuing one responder request at a time. `FCGI_KEEP_CONN` is
 * set, so the connection is reused where the SAPI honours it and reopened where it does not, and
 * `reconnects` counts the second case for `HttpConn`'s reason.
 */
export class FcgiConn implements Conn {
  reconnects = 0;
  private wire!: Wire;
  private readonly wireRequest: Buffer;

  private constructor(private port: number, script: string) {
    const params = Object.entries({
      GATEWAY_INTERFACE: "CGI/1.1",
      REQUEST_METHOD: "GET",
      SCRIPT_FILENAME: script,
      SCRIPT_NAME: `/${basename(script)}`,
      REQUEST_URI: `/${basename(script)}`,
      DOCUMENT_ROOT: dirname(script),
      QUERY_STRING: "",
      SERVER_PROTOCOL: "HTTP/1.1",
      SERVER_SOFTWARE: "novis-bench",
      SERVER_NAME: "127.0.0.1",
      REMOTE_ADDR: "127.0.0.1",
      CONTENT_LENGTH: "0",
    }).map(([k, v]) => fcgiPair(k, v));
    const begin = Buffer.alloc(8);
    begin.writeUInt16BE(FCGI_RESPONDER, 0);
    begin.writeUInt8(FCGI_KEEP_CONN, 2);
    this.wireRequest = Buffer.concat([
      fcgiRecord(FCGI_BEGIN_REQUEST, begin),
      fcgiRecord(FCGI_PARAMS, Buffer.concat(params)),
      fcgiRecord(FCGI_PARAMS, EMPTY),
      fcgiRecord(FCGI_STDIN, EMPTY),
    ]);
  }

  static async open(port: number, script: string): Promise<FcgiConn> {
    const c = new FcgiConn(port, script);
    c.wire = await Wire.open(port, CONNECT_TIMEOUT_MS);
    return c;
  }

  async request(): Promise<Buffer> {
    try {
      return await this.exchange();
    } catch (e) {
      if (!(e instanceof Closed)) throw e;
      // The SAPI declined FCGI_KEEP_CONN; that is a measurement, not a failure.
      this.reconnects++;
      this.wire.close();
      this.wire = await Wire.open(this.port, CONNECT_TIMEOUT_MS);
      return await this.exchange();
    }
  }

  private async exchange(): Promise<Buffer> {
    const w = this.wire;
    const closed = "php-cgi closed the connection mid-record";
    w.write(this.wireRequest);
    const out: Buffer[] = [];
    const err: Buffer[] = [];
    for (;;) {
      const head = await w.take(8, closed);
      const kind = head.readUInt8(1);
      const length = head.readUInt16BE(4);
      const body = Buffer.from(await w.take(length + head.readUInt8(6), closed)).subarray(0, length);
      if (kind === FCGI_STDOUT) out.push(body);
      else if (kind === FCGI_STDERR) err.push(body);
      else if (kind === FCGI_END_REQUEST) break;
    }
    if (err.length > 0) throw new Error(`php-cgi wrote to stderr: ${Buffer.concat(err).toString("utf8").slice(0, 300)}`);
    const all = Buffer.concat(out);
    const at = all.indexOf("\r\n\r\n");
    return at < 0 ? all : all.subarray(at + 4);
  }

  close(): void {
    this.wire.close();
  }
}

/** A port the OS has just said is free. Racy by nature, as every harness is. */
export function freePort(): Promise<number> {
  return new Promise((ok, fail) => {
    const probe = createServer();
    probe.once("error", fail);
    probe.listen(0, "127.0.0.1", () => {
      const { port } = probe.address() as AddressInfo;
      probe.close(() => ok(port));
    });
  });
}

/**
 * A server subprocess, listening, with its output drained. The drain is not tidiness: `php -S` logs a
 * line per request, and an undrained pipe wedges it at the OS buffer size, which reads as the peer
 * becoming slow halfway through a run.
 */
export class Server {
  private log: string[] = [];
  private drained: Promise<unknown>;

  private constructor(private argv: string[], private proc: Bun.Subprocess<"ignore", "pipe", "pipe">) {
    this.drained = Promise.all([this.drain(proc.stdout), this.drain(proc.stderr)]);
  }

  static async start(argv: string[], port: number, cwd?: string, env?: Record<string, string>): Promise<Server> {
    const proc = Bun.spawn(argv, { cwd: cwd ?? ROOT, env: env ? { ...process.env, ...env } : process.env, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
    const server = new Server(argv, proc);
    await server.await(port);
    return server;
  }

  private async drain(stream: ReadableStream<Uint8Array>): Promise<void> {
    const decoder = new TextDecoder();
    let pending = "";
    for await (const chunk of stream) {
      const lines = (pending + decoder.decode(chunk, { stream: true })).split(/\r?\n/);
      pending = lines.pop()!;
      for (const line of lines) this.keep(line);
    }
    if (pending) this.keep(pending);
  }

  private keep(line: string): void {
    this.log.push(line.trimEnd());
    if (this.log.length > 60) this.log.shift();
  }

  said(): string {
    return this.log.map((l) => `    ${l}`).join("\n") || "    (nothing)";
  }

  private async await(port: number): Promise<void> {
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
      if (this.proc.exitCode !== null) {
        await Promise.race([this.drained, Bun.sleep(1000)]);
        throw new Error(`${this.argv[0]} exited ${this.proc.exitCode} before it listened:\n${this.said()}`);
      }
      try {
        (await Wire.open(port, 500)).close();
        return;
      } catch {
        await Bun.sleep(50);
      }
    }
    await this.stop();
    throw new Error(`${this.argv[0]} did not listen on port ${port} within 30s:\n${this.said()}`);
  }

  async stop(): Promise<void> {
    this.proc.kill();
    await Promise.race([this.proc.exited, Bun.sleep(10_000)]);
    if (this.proc.exitCode === null && this.proc.signalCode === null) {
      this.proc.kill("SIGKILL");
      await this.proc.exited;
    }
  }
}

/**
 * `requests` GETs over `concurrency` keep-alive connections: seconds, the body, and the reconnects.
 * Closed loop -- each connection issues its next request the moment the previous answer is in hand.
 * The connections open before the clock starts.
 */
export async function hammer(open: () => Promise<Conn>, requests: number, concurrency: number): Promise<{ elapsed: number; body: Buffer; reopened: number }> {
  const share = Array.from({ length: concurrency }, (_, i) => Math.floor(requests / concurrency) + (i < requests % concurrency ? 1 : 0));
  const conns: Conn[] = [];
  for (const _ of share) conns.push(await open());
  const bodies: Buffer[] = share.map(() => EMPTY);
  const errors: unknown[] = share.map(() => null);
  const start = performance.now();
  await Promise.all(
    share.map(async (n, i) => {
      try {
        for (let k = 0; k < n; k++) bodies[i] = await conns[i]!.request();
      } catch (e) {
        errors[i] = e;
      }
    }),
  );
  const elapsed = (performance.now() - start) / 1000;
  for (const c of conns) c.close();
  for (const e of errors) if (e !== null) throw e;
  const answered = [...new Set(bodies.filter((b) => b.length > 0).map((b) => b.toString("latin1")))];
  if (answered.length > 1) throw new Error("one server answered two connections with two different bodies");
  const reopened = conns.reduce((n, c) => n + c.reconnects, 0);
  return { elapsed, body: answered.length ? Buffer.from(answered[0]!, "latin1") : EMPTY, reopened };
}

interface Peer {
  kind: string;
  label: string;
  detail: string;
  argv: string[];
  cwd?: string;
  env?: Record<string, string>;
  port: number;
  executable: string;
  versionArgv: string[];
  open: () => Promise<Conn>;
}

interface ServerResult {
  reconnects: number;
  kind: string;
  label: string;
  detail: string;
  requests_per_sec: number;
  median_requests_per_sec: number;
  ms_per_request: number;
  version: string;
  body: Buffer;
}

const messageOf = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Boots one peer, warms it, times `reps` passes over it, and shuts it down. */
async function measureServer(peer: Peer, requests: number, concurrency: number, reps: number): Promise<ServerResult> {
  const server = await Server.start(peer.argv, peer.port, peer.cwd, peer.env);
  const rates: number[] = [];
  let body = EMPTY;
  let reopened = 0;
  try {
    // Discarded: the page cache, opcache's first compile and the JIT's first trace are start-up costs,
    // and this leg measures a server that has been up for a while.
    await hammer(peer.open, Math.max(20, Math.min(requests, 200)), concurrency);
    for (let i = 0; i < reps; i++) {
      const pass = await hammer(peer.open, requests, concurrency);
      rates.push(requests / pass.elapsed);
      body = pass.body;
      reopened += pass.reopened;
    }
  } catch (e) {
    const hint =
      concurrency > 1
        ? `; at --concurrency ${concurrency} a single-process peer does not answer the second connection at all, ` +
          "which arrives here as a timeout -- the module doc says why 1 is the default"
        : "";
    // The generator only ever sees a reset socket; the sentence that says why is on the peer's own output.
    throw new Error(`${peer.label} failed mid-run (${messageOf(e)})${hint}; it said:\n${server.said()}`);
  } finally {
    await server.stop();
  }
  const best = Math.max(...rates);
  return {
    reconnects: reopened,
    kind: peer.kind,
    label: peer.label,
    detail: peer.detail,
    requests_per_sec: best,
    median_requests_per_sec: median(rates),
    ms_per_request: (1000 * concurrency) / best,
    version: version(peer.executable, peer.versionArgv),
    body,
  };
}

/**
 * The best PHP peer this box offers, in the order the module doc argues for. `choice` names one instead,
 * so the two weaker branches are reachable on a box that has the strong one: a fallback nothing can run
 * is a fallback nobody has run.
 */
async function phpPeer(php: string, phpMode: string, choice: string): Promise<Peer | null> {
  const script = join(SERVE_DIR, `${SERVE_CASE}.php`);
  const opcache = ["-d", "opcache.enable=1", "-d", "opcache.enable_cli=1", ...PHP_MODES[phpMode]!];
  if (choice === "none") return null;
  const cgi = choice === "auto" || choice === "fcgi" ? Bun.which("php-cgi") : null;
  if (cgi) {
    const port = await freePort();
    return {
      kind: "php-cgi-fastcgi",
      label: "php-cgi",
      detail: "the FastCGI SAPI FPM runs, opcache on",
      argv: [cgi, "-b", `127.0.0.1:${port}`, ...opcache],
      // The FastCGI SAPI exits after 500 requests unless told otherwise, which arrives here as a reset
      // socket partway into a run. Recycling is a leak workaround, and a benchmark that recycles
      // measures process start-up.
      env: { PHP_FCGI_MAX_REQUESTS: "0" },
      port,
      executable: cgi,
      versionArgv: ["-v"],
      open: () => FcgiConn.open(port, script),
    };
  }
  if ((choice === "auto" || choice === "builtin") && Bun.which(php)) {
    const port = await freePort();
    return {
      kind: "php-builtin-server",
      label: "php -S",
      detail: "PHP's own development server, serial by construction",
      argv: [php, ...opcache, "-S", `127.0.0.1:${port}`, script],
      port,
      executable: php,
      versionArgv: ["-v"],
      open: () => HttpConn.open(port),
    };
  }
  return null;
}

/** One peer's half of the record: the numbers, never the body it sent. */
function serveEntry(r: ServerResult): Record<string, unknown> {
  return {
    kind: r.kind,
    label: r.label,
    detail: r.detail,
    version: r.version,
    requests_per_sec: round(r.requests_per_sec, 1),
    median_requests_per_sec: round(r.median_requests_per_sec, 1),
    ms_per_request: round(r.ms_per_request, 4),
    reconnects: r.reconnects,
  };
}

const pyTypeName = (v: unknown) =>
  v === null ? "NoneType" : typeof v === "string" ? "str" : typeof v === "boolean" ? "bool" : typeof v === "number" ? (Number.isInteger(v) ? "int" : "float") : "dict";

/**
 * Appends one run to a JSON array, so the artifact stays a readable history that parses as a whole. The
 * history keeps the last `SERVE_HISTORY` runs, because the loop's acceptance check appends one every
 * iteration and the run that matters is the most recent one on a given box.
 *
 * Where the caller points this is what makes the artifact. The loop's sweep records into the ignored
 * `benches/results/`, because a row appended after a session's commits is a change no slice owns;
 * `benches/serve.json` is tracked, so a row there is a figure published by hand.
 * `docs/agent/commands.md` § *The server's throughput* is that split's home.
 */
export function writeServeRecord(path: string, record: Record<string, unknown>): void {
  const full = resolve(path);
  mkdirSync(dirname(full), { recursive: true });
  let history: unknown[] = [];
  if (existsSync(full) && statSync(full).size > 0) {
    let parsed: unknown;
    try {
      parsed = JSON.parse(readFileSync(full, "utf8"));
    } catch (e) {
      throw new Fail(`${shownPath(path)} is not the JSON array this leg appends to (${messageOf(e)}); move it aside`);
    }
    if (!Array.isArray(parsed)) throw new Fail(`${shownPath(path)} holds a ${pyTypeName(parsed)}, not the JSON array this leg appends to`);
    history = parsed;
  }
  history = [...history, record].slice(-SERVE_HISTORY);
  writeFileSync(full, JSON.stringify(history, null, 2) + "\n", "utf8");
  const shown = isAbsolute(path) && !relative(ROOT, full).startsWith("..") ? relative(ROOT, full) : shownPath(path);
  console.log(`  recorded 1 run to ${shown} (${history.length} in the history)`);
}

/** M7's throughput figure: `nvs serve` against PHP with opcache, both under one generator. */
async function serveVsFpm(binary: string, o: Options, reps: number): Promise<number> {
  const entry = join(SERVE_DIR, `${SERVE_CASE}.nvs`);
  const twin = join(SERVE_DIR, `${SERVE_CASE}.php`);
  const isFile = (p: string) => existsSync(p) && statSync(p).isFile();
  if (!isFile(entry)) throw new Fail(`${entry} is missing; this leg's case is the twin pair under benches/serve`);
  if (o.requests < 1 || o.concurrency < 1) throw new Fail("--requests and --concurrency must each be at least 1");

  const port = await freePort();
  const exe = resolve(binary);
  const novis: Peer = {
    kind: "nvs-serve",
    label: "nvs serve",
    detail: "`rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s development server, one core",
    argv: [exe, "serve", entry, "--listen", `127.0.0.1:${port}`],
    cwd: ROOT,
    port,
    executable: exe,
    versionArgv: ["--version"],
    open: () => HttpConn.open(port),
  };
  const peer = isFile(twin) ? await phpPeer(o.php, o.phpMode, o.serveBaseline) : null;
  if (peer !== null && peer.kind === "php-cgi-fastcgi" && o.concurrency > 1) {
    // Honoured where FPM would exist and ignored on Windows, where the SAPI has no fork to make.
    peer.env!.PHP_FCGI_CHILDREN = String(o.concurrency);
  }

  const caveats: string[] = [];
  if (peer === null && !isFile(twin)) caveats.push(`no PHP baseline: ${basename(twin)} is missing`);
  else if (peer === null && o.serveBaseline === "none") caveats.push("no PHP baseline: --serve-baseline none, so this run is nvs serve alone");
  else if (peer === null) caveats.push(`no PHP baseline: --serve-baseline ${o.serveBaseline} found nothing on PATH, so this run is nvs serve alone`);
  else if (peer.kind === "php-builtin-server") {
    caveats.push(
      "the baseline is `php -S`, PHP's development server -- a far weaker peer than the FastCGI SAPI FPM runs, and a ratio against it is not M7's figure",
    );
  } else {
    caveats.push(
      "the baseline pays no HTTP parse and no proxy hop -- it is driven over FastCGI as nginx would drive FPM -- while nvs serve pays both; the comparison favours PHP",
    );
  }
  if (o.concurrency > 1) {
    caveats.push(`concurrency is ${o.concurrency}: neither PHP peer here serves two requests at once, so anything above 1 is measuring Novis against a queue`);
  }

  console.log(`case ${SERVE_CASE}: ${o.requests} request(s) over ${o.concurrency} keep-alive connection(s), ${reps} rep(s), best of reps`);
  const measured = [await measureServer(novis, o.requests, o.concurrency, reps)];
  if (peer !== null) measured.push(await measureServer(peer, o.requests, o.concurrency, reps));
  console.log();

  const labelWidth = Math.max(...measured.map((m) => m.label.length));
  for (const r of measured) {
    console.log(
      `  ${r.label.padEnd(labelWidth)}  ${grouped(r.requests_per_sec, 0).padStart(10)} requests/sec` +
        `  ${fixed(r.ms_per_request, 3).padStart(8)} ms/request   ${r.detail}`,
    );
  }

  if (new Set(measured.map((m) => m.body.toString("latin1"))).size > 1) {
    console.log();
    console.log("DIFF: the two servers did not answer with the same bytes, so neither number stands");
    for (const r of measured) console.log(`  ${r.label} sent ${bytesRepr(r.body)}`);
    return 1;
  }

  for (const r of measured) {
    if (r.reconnects) caveats.push(`${r.label} declined to keep the connection open and paid ${r.reconnects} extra handshake(s) inside the timed passes`);
  }

  let ratio: number | null = null;
  if (measured.length === 2) {
    ratio = measured[0]!.requests_per_sec / measured[1]!.requests_per_sec;
    console.log(`  nvs serve is ${fixed(ratio, 2)}x ${measured[1]!.label} at concurrency ${o.concurrency}`);
  }
  console.log();
  for (const caveat of caveats) console.log(`  note: ${caveat}`);

  const record = {
    date: stamp(),
    commit: version("git", ["rev-parse", "--short", "HEAD"]),
    case: SERVE_CASE,
    requests: o.requests,
    concurrency: o.concurrency,
    reps,
    php_mode: o.phpMode,
    ratio: ratio === null ? null : round(ratio, 4),
    caveats,
    nvs: serveEntry(measured[0]!),
    baseline: measured.length === 2 ? serveEntry(measured[1]!) : null,
    nvs_binary: repoRelative(binary),
    host: host(),
  };
  if (o.record) writeServeRecord(o.record, record);
  else console.log("  not recorded: pass --record PATH to append this run to a JSON array");
  return 0;
}

// ---------------------------------------------------------------------------------------------------

async function main(o: Options): Promise<number> {
  if (o.maxMs !== null) {
    // Retired loudly rather than silently: a `--max-ms 10` left in a goal would go on budgeting the whole
    // wall clock, most of which is the operating system.
    throw new Fail(
      "--max-ms budgeted the whole wall clock, of which most is the operating system creating a process " +
        "rather than anything Novis does. Use --max-work-ms, which budgets the total less `nvs --version`; " +
        "`--warm-start` in `nv bench` says why.",
    );
  }
  if (o.reps !== null && o.reps < 1) throw new Fail("--reps must be at least 1");
  const askedReps = o.reps ?? (o.warmStart ? WARM_START_REPS : SUITE_REPS);
  const reps = o.check ? 1 : askedReps;

  const binary = findNvs(o.nvs, o.allowDebug);
  warnIfStale(binary);
  // Both legs before the roster, because each is one engine's figure and must not need PHP, Python or
  // Bun installed to be measured. Each takes `askedReps`: `--check` narrows the suite to agreement, and
  // neither leg has anything to agree with.
  if (o.warmStart) return warmStart(binary, askedReps, o.maxWorkMs);
  if (o.serveVsFpm) return serveVsFpm(binary, o, askedReps);
  if (o.record) throw new Fail("--record writes the serve leg's artifact, and needs --serve-vs-fpm");
  if (o.maxWorkMs !== null) throw new Fail("--max-work-ms is a budget on the warm-start figure, and needs --warm-start");
  return suite(binary, o, reps);
}

export async function run(args: string[]): Promise<number> {
  let o: Options | null;
  try {
    o = options(args);
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv bench: error: ${e.message}`);
    return 2;
  }
  if (o === null) {
    console.log(help());
    return 0;
  }
  try {
    return await main(o);
  } catch (e) {
    if (!(e instanceof Fail)) throw e;
    console.error(e.message);
    return 1;
  }
}
