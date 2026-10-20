// `bun nv bench`: the CLI's warm-start figure, what a second `nvs run` of the empty program costs.
//
//     bun nv bench --warm-start                    the figure, over 25 timed reps
//     bun nv bench --warm-start --reps 49          more reps when the figure looks noisy
//     bun nv bench --warm-start --max-work-ms 9    with a soft bound, and ten times it as the hard one
//
// The figure is the command's only leg. `--warm-start` names it, so CI's command line says what it
// measures, and a run without the flag measures the same figure.
//
// # What is measured
//
// Wall clock of the whole process, the minimum over N reps after one discarded warm-up. Every source of
// noise on a desktop adds time and none subtracts it, so the minimum is the estimate of the work, and
// the median is printed beside it so a busy machine shows as a gap between the two.
//
// The program is the empty one, `WARM_START_PROGRAM`, written under `target/` on each run where it is
// not already there byte for byte. The discarded warm-up is what makes the figure warm: the binary and
// the script are in the page cache, and the artifact that run published is what every timed rep loads,
// so the figure covers `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`'s read
// path and not a compile. A cache directory that is refused makes every rep compile, which is the shape
// a regression here takes.
//
// A binary under `target/debug/` is refused unless `--allow-debug` says the caller means it, since a
// debug build measures its assertions. Nothing here builds, and a warning says so when the binary is
// older than the newest file under `crates/`.
//
// # The warm-start figure
//
// `--max-work-ms` bounds the total LESS `nvs --version`. Most of a process's wall clock is the
// operating system creating it, and a bound on the total bounds the machine. `--version` is the same
// binary, loader and page cache doing strictly less, so the difference is config load, compile and run.
// What this cannot see is a regression in the floor itself, which is why both figures are printed on
// every run.
//
// The figure has the two bounds `rule:testing/perf-two-mechanisms` gives a timing. `--max-work-ms` is
// the soft one: a figure over it prints a warning, a GitHub Actions annotation and a step-summary line,
// and fails nothing. A hosted Linux runner reads 5 to 8 ms of work across runs of commits with no
// start-up change between them, so a single bound at 6 ms failed about every second run, and CI
// passes 9. The hard bound is `HARD_FACTOR` times the soft one, the rule's order of magnitude: a cache
// miss cannot be the mark, because the empty program compiles in so little time that a warm run costs
// 85 to 93% of a cold one. A figure past the hard bound is measured again, up to `HARD_ATTEMPTS`
// attempts, and only the last is judged, so it fails on a start-up that changed in kind and not on a
// burst of load.
//
// The soft bound is read only on a quiet box. Load on a machine dilates a process multiplicatively, so
// the level of the start floor is the reading that tells a busy box from a quiet one: above
// `QUIET_FLOOR_MS` the leg says the soft bound was not read. The hard bound is judged on every box.
//
// The configuration is named rather than left to `./nvs.toml`, which in this repository is a fixture the
// goals keep adding to. `SHIPPED_CONFIG` is what a user's start resolves, so a directive added to the
// product moves the figure and one added to this tree's housekeeping does not. The leg takes
// `WARM_START_REPS` because a minimum over a handful of samples is biased high, and that bias lands on
// a difference of two small numbers.
//
// # What the other benches borrow
//
// The load leg (`tools/nv/cmd/bench-load.ts`) imports the binary lookup, the stale-build warning, the
// free port, the server subprocess, the version probe and the record writer from here, and the proxied
// leg (`tools/nv/cmd/bench-proxied.ts`) imports the FastCGI client and the closed-loop generator as
// well. `bun nv scaling`'s `serve` ladders import the free port, the server subprocess with its peak
// memory, the HTTP client and the generator. Each of them has one copy to keep correct. A record's
// `host` names the machine the way Bun does, and a float with no fraction is written `12`.

import { createServer, connect, type AddressInfo, type Socket } from "node:net";
import { cpus, machine, release } from "node:os";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync, appendFileSync } from "node:fs";
import { basename, dirname, isAbsolute, join, normalize, relative, resolve, sep } from "node:path";
import { signalOwned, Tree } from "../lib/proctree.ts";
import { ROOT } from "../lib/paths.ts";
import { ArgError, fixed, parseArgs, pyInt, pyRepr, splitlines } from "../lib/py.ts";
import { unrecorded } from "../lib/reads.ts";

export const summary = "the CLI's warm-start figure: nv bench --warm-start [--max-work-ms MS]";

const USAGE = [
  "usage: nv bench [-h] [--warm-start] [--reps REPS] [--nvs NVS] [--allow-debug]",
  "                [--max-work-ms MS]",
].join("\n");

const WINDOWS = process.platform === "win32";
/** The configuration the figure is measured against; the module doc says why it is named. */
const SHIPPED_CONFIG = join(ROOT, "crates", "nvs-config", "src", "default.toml");
/** Where the empty program is written; `target/` is build output, and never committed. */
const WARM_START_SOURCE = join(ROOT, "target", "nv-bench", "warm-start.nvs");
/** The empty program: what `nvs run` costs before any of a script's own code runs. */
const WARM_START_PROGRAM = '<?nvs\necho "warm start\\n";\n';
/** The figure reads no soft bound above this start floor; the module doc says why. */
const QUIET_FLOOR_MS = 5.75;
/** The hard bound is this many times `--max-work-ms`, the soft one. */
const HARD_FACTOR = 10;
/** How many times a figure past its hard bound is measured before it fails. */
const HARD_ATTEMPTS = 3;
const WARM_START_REPS = 25;
/** Runs kept in a record file; `writeServeRecord` says why it is capped. */
const SERVE_HISTORY = 100;

/** A message the command stops on, printed alone to stderr with exit status 1. */
export class Fail extends Error {}

// ---------------------------------------------------------------------------------------------------
// The command line.
// ---------------------------------------------------------------------------------------------------

interface Options {
  reps: number | null;
  nvs: string | null;
  allowDebug: boolean;
  maxWorkMs: number | null;
  maxMs: number | null;
}

const FLAGS = ["--allow-debug", "--warm-start"];
const VALUED = ["--reps", "--nvs", "--max-work-ms", "--max-ms"];
const ORDER = ["--warm-start", "--reps", "--nvs", "--allow-debug", "--max-work-ms", "--max-ms"];

/** Python's `float` over a command-line word, or null where it would raise. */
function pyFloat(word: string): number | null {
  const w = word.trim().replaceAll(/(?<=\d)_(?=\d)/g, "");
  if (/^[+-]?(inf|infinity)$/i.test(w)) return w.startsWith("-") ? -Infinity : Infinity;
  if (/^[+-]?nan$/i.test(w)) return NaN;
  return /^[+-]?(\d+\.?\d*|\.\d+)(e[+-]?\d+)?$/i.test(w) ? Number(w) : null;
}

/** The options argparse would have read, or null when `--help` asked for the help instead. */
function options(args: string[]): Options | null {
  const p = parseArgs(args, { flags: FLAGS, valued: VALUED, order: ORDER, positionals: false });
  if (p.flags.has("--help")) return null;
  const typed = (o: string, kind: "int" | "float", convert: (w: string) => number | null): number | null => {
    const word = p.values.get(o);
    if (word === undefined) return null;
    const n = convert(word);
    if (n === null) throw new ArgError(`argument ${o}: invalid ${kind} value: ${pyRepr(word)}`);
    return n;
  };
  return {
    reps: typed("--reps", "int", pyInt),
    nvs: p.values.get("--nvs") ?? null,
    allowDebug: p.flags.has("--allow-debug"),
    maxWorkMs: typed("--max-work-ms", "float", pyFloat),
    maxMs: typed("--max-ms", "float", pyFloat),
  };
}

function help(): string {
  return [
    USAGE,
    "",
    "Measure how long `nvs run` takes to start an empty program when its compiled code is already cached.",
    "The figure is the total time less the time of `nvs --version`.",
    "",
    "options:",
    "  -h, --help            show this help message and exit",
    "  --warm-start          measure the warm-start figure (this is also what runs without the flag)",
    `  --reps REPS           timed reps (default ${WARM_START_REPS})`,
    "  --nvs NVS             path to the nvs binary (default target/release)",
    "  --allow-debug         allow a debug nvs binary",
    `  --max-work-ms MS      print a warning if the figure is over MS, and fail if it is over ${HARD_FACTOR} times MS`,
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

/** Text mode's newline translation, so two runs on one platform compare as Python compared them. */
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

/** One command, warmed once and then timed `reps` times; a failed or unstable run is `failed`. */
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

/** One warm-start reading: the start floor, the warm run and the work between them. */
interface WarmReading {
  floor: Measured;
  total: Measured;
  work: number;
}

/** Measures the floor and the warm run once each, or prints why one failed and gives null. */
function warmReading(exe: string, source: string, reps: number): WarmReading | null {
  const floor = measure([exe, "--version"], reps);
  const total = measure([exe, "run", "--config", SHIPPED_CONFIG, source], reps);
  for (const [what, result] of [["nvs --version", floor], ["nvs run", total]] as const) {
    if (result.failed) {
      console.log(`warm start FAILED: \`${what}\` exit ${result.code}`);
      console.error(result.stderr.trimEnd() || result.stdout.trimEnd());
      return null;
    }
  }
  return { floor, total, work: total.min_ms! - floor.min_ms! };
}

/** A soft-bound miss: a line in the log, and on GitHub Actions an annotation and a line in the step
 * summary (`rule:testing/perf-two-mechanisms`). It fails nothing. */
export function softWarning(text: string): void {
  console.log(`  warning: ${text}`);
  if (process.env.GITHUB_ACTIONS !== "true") return;
  console.log(`::warning title=warm start::${text}`);
  const summary = process.env.GITHUB_STEP_SUMMARY;
  if (summary) appendFileSync(summary, `- warm start: ${text}\n`);
}

/** `WARM_START_SOURCE`, written only where it is missing or differs, so a rerun leaves it untouched. */
function emptyProgram(): string {
  const current = existsSync(WARM_START_SOURCE) ? readFileSync(WARM_START_SOURCE, "utf8") : null;
  if (current !== WARM_START_PROGRAM) {
    mkdirSync(dirname(WARM_START_SOURCE), { recursive: true });
    writeFileSync(WARM_START_SOURCE, WARM_START_PROGRAM, "utf8");
  }
  return WARM_START_SOURCE;
}

/** What a second `nvs run` of the empty program costs, less what starting the binary at all costs.
 * `maxWorkMs` is the soft bound and `HARD_FACTOR` times it the hard one; the module doc says why. */
function warmStart(binary: string, reps: number, maxWorkMs: number | null): number {
  if (!existsSync(SHIPPED_CONFIG)) throw new Fail(`no ${SHIPPED_CONFIG}: the warm-start figure names the configuration it resolves`);
  const source = emptyProgram();

  const exe = resolve(binary);
  console.log(`nvs  ${shownPath(binary)}  (${version(exe, ["--version"])})`);
  const hardMs = maxWorkMs === null ? null : maxWorkMs * HARD_FACTOR;
  let reading = warmReading(exe, source, reps);
  // Past the hard bound, the figure is measured again and only the last attempt is judged.
  for (let attempt = 1; reading !== null && hardMs !== null && reading.work > hardMs && attempt < HARD_ATTEMPTS; attempt++) {
    console.log(`  attempt ${attempt} of ${HARD_ATTEMPTS}: ${fmt(reading.work)} ms of work is past the ${fmt(hardMs)} ms hard bound, measuring again`);
    reading = warmReading(exe, source, reps);
  }
  if (reading === null) return 1;
  const { floor, total, work } = reading;

  const label = relative(ROOT, source).split(sep).join("/");
  const config = relative(ROOT, SHIPPED_CONFIG).split(sep).join("/");
  console.log(`warm start  nvs run ${label}  (${reps} rep(s), one discarded warm-up)`);
  console.log(`  start floor ${fmt(floor.min_ms)} ms   nvs --version, the OS creating a process`);
  console.log(`  total       ${fmt(total.min_ms)} ms   median ${fmt(total.median_ms)} ms`);
  console.log(`  novis work  ${fmt(work)} ms   the total less that floor`);
  console.log(`  config      ${config}, so the figure moves with the binary rather than with this tree's own \`nvs.toml\``);
  console.log("  each rep is an `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` warm hit; the discarded warm-up published the artifact");
  if (maxWorkMs === null || hardMs === null) return 0;
  if (work > hardMs) {
    console.log(`  hard bound ${fmt(hardMs)} ms on the work -- EXCEEDED on all ${HARD_ATTEMPTS} attempts`);
    return 1;
  }
  console.log(`  hard bound ${fmt(hardMs)} ms on the work -- within`);
  if (floor.min_ms! > QUIET_FLOOR_MS) {
    console.log(
      `  soft bound not read -- \`nvs --version\` alone costs ${fmt(floor.min_ms)} ms here, over the ${fmt(QUIET_FLOOR_MS)} ms ` +
        `a box this bound can be read on, so the ${fmt(work)} ms difference is a figure about the machine`,
    );
    return 0;
  }
  if (work > maxWorkMs) {
    softWarning(`${fmt(work)} ms of work is over the ${fmt(maxWorkMs)} ms soft bound; measure the commit before it on this machine`);
  } else console.log(`  soft bound ${fmt(maxWorkMs)} ms on the work -- within`);
  return 0;
}

// ---------------------------------------------------------------------------------------------------
// The HTTP and FastCGI clients, the server subprocess and the closed-loop generator. Nothing in this
// command uses them: they are here for the benches the module doc names.
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

/** What `HttpConn` sends beyond a bare GET: extra header lines, and a body sent with `POST`. */
export interface HttpShape {
  headers?: [string, string][];
  body?: Buffer;
}

/**
 * One keep-alive HTTP/1.1 connection, issuing one request at a time: a GET, or a POST when the shape
 * carries a body. It reopens when the peer answers `Connection: close`, and `reconnects` counts it: a
 * peer paying a handshake per request and one that is not are two different measurements.
 */
export class HttpConn implements Conn {
  reconnects = 0;
  private wire!: Wire;
  private readonly wireRequest: Buffer;

  private constructor(private port: number, path: string, shape: HttpShape) {
    const extra = (shape.headers ?? []).map(([k, v]) => `${k}: ${v}\r\n`).join("");
    const head = shape.body
      ? `POST ${path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: */*\r\n${extra}Content-Type: text/plain\r\nContent-Length: ${shape.body.length}\r\n\r\n`
      : `GET ${path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: */*\r\n${extra}\r\n`;
    this.wireRequest = shape.body ? Buffer.concat([Buffer.from(head, "latin1"), shape.body]) : Buffer.from(head, "latin1");
  }

  static async open(port: number, path = "/", shape: HttpShape = {}): Promise<HttpConn> {
    const c = new HttpConn(port, path, shape);
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
      // HTTP/1.0 framing: the body ends when the peer closes.
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
 * A server subprocess, listening, with its output drained. The drain is not tidiness: a server that
 * logs a line per request wedges at the OS buffer size on an undrained pipe, which reads as the peer
 * becoming slow halfway through a run.
 */
export class Server {
  private log: string[] = [];
  private drained: Promise<unknown>;

  private constructor(private argv: string[], private proc: Bun.Subprocess<"ignore", "pipe", "pipe">, private tree: Tree) {
    this.drained = Promise.all([this.drain(proc.stdout), this.drain(proc.stderr)]);
  }

  static async start(argv: string[], port: number, cwd?: string, env?: Record<string, string>): Promise<Server> {
    const proc = Bun.spawn(argv, { cwd: cwd ?? ROOT, env: env ? { ...process.env, ...env } : process.env, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
    const server = new Server(argv, proc, new Tree(proc));
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
        await this.stop();
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

  /** Asks the server to stop, then after ten seconds kills it and every process it started that is still running. */
  async stop(): Promise<void> {
    signalOwned(this.proc);
    await Promise.race([this.proc.exited, Bun.sleep(10_000)]);
    this.tree.kill();
    await this.proc.exited;
    this.tree.close();
  }

  /** The most memory the server process held at once, in bytes, read after `stop`; null where the OS does not say. */
  peakMemory(): number | null {
    return this.proc.resourceUsage()?.maxRSS ?? null;
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

const messageOf = (e: unknown) => (e instanceof Error ? e.message : String(e));

const pyTypeName = (v: unknown) =>
  v === null ? "NoneType" : typeof v === "string" ? "str" : typeof v === "boolean" ? "bool" : typeof v === "number" ? (Number.isInteger(v) ? "int" : "float") : "dict";

/**
 * Appends one run to a JSON array, so the artifact stays a readable history that parses as a whole. The
 * history keeps the last `SERVE_HISTORY` runs, because the loop's acceptance check appends one every
 * iteration and the run that matters is the most recent one on a given box.
 *
 * Where the caller points this is what makes the artifact. The loop's sweep records into the ignored
 * `benches/results/`, because a row appended after a session's commits is a change no slice owns;
 * a tracked file such as `benches/serve-proxied.json` holds only figures published by hand.
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

// ---------------------------------------------------------------------------------------------------

async function main(o: Options): Promise<number> {
  if (o.maxMs !== null) {
    // Retired loudly rather than silently: a `--max-ms 10` left in a goal would go on budgeting the whole
    // wall clock, most of which is the operating system.
    throw new Fail(
      "--max-ms budgeted the whole wall clock, of which most is the operating system creating a process " +
        "rather than anything Novis does. Use --max-work-ms, which budgets the total less `nvs --version`; " +
        "`nv bench --help` says what the figure is.",
    );
  }
  if (o.reps !== null && o.reps < 1) throw new Fail("--reps must be at least 1");
  const binary = findNvs(o.nvs, o.allowDebug);
  warnIfStale(binary);
  return warmStart(binary, o.reps ?? WARM_START_REPS, o.maxWorkMs);
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
