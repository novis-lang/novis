// `bun nv bench-load`: the load leg -- what `nvs serve` saturates at on this machine, with every answer
// verified.
//
//     bun nv bench-load                                         # the sweep, then the in-flight leg
//     bun nv bench-load --record benches/serve-load.json        # append the run (check-links:written)
//     bun nv bench-load --concurrency 1,16,256                  # drive exactly these widths
//     bun nv bench-load --seconds 30                            # longer points, for a figure worth keeping
//     bun nv bench-load --in-flight 0                           # the sweep alone
//     bun nv bench-load --probe path/to/nvs-serve-probe         # skip the cargo build
//
// `benches/serve-probe/` is the generator and its module doc owns the client half -- the token every
// request carries, the two modes, and what a run does not measure. This file owns only how a run is
// carried out on whatever machine it is sitting on.
//
// # What this leg is, and why it is a third one
//
// The three serve legs answer three different questions and no arithmetic crosses between them:
//
// * `bun nv bench --serve-vs-fpm` compares against PHP on this box with no proxy. It is goal `server`'s
//   acceptance check, it must keep working where there is no Docker and no `wrk`, and its generator is
//   written into that command so it needs nothing built.
// * `bun nv bench-proxied` puts nginx in front of both peers in containers, which is the only
//   deployment either has.
// * **This one has no peer at all.** It asks what this machine's server saturates at, and -- the part
//   neither other leg can answer -- whether every response was the answer to the request that asked for
//   it. Both of those need a generator faster than the server, which is why this leg is the one with
//   something to build.
//
// **A throughput figure over one constant response is not evidence of a correct server.** Both other legs
// drive `hello.nvs`, whose every answer is byte-identical, so a response delivered to the wrong connection
// is indistinguishable from the right one and no generator can see it. This leg drives
// `benches/serve/echo.nvs`, whose body is the token the request carried, and the probe rejects an answer
// that names a different request. `nvs-serve-probe`'s own `a_constant_body_fails_every_request` is what
// holds the check honest: point the verifier at the constant case and every request must fail.
//
// # Why the sweep is derived from the core count and not written down
//
// A fixed concurrency list measures a different thing on every machine -- 64 connections is past
// saturation on four cores and short of it on sixty-four. So the default straddles *this* box: one
// connection, one per core, and multiples of the core count out to where the queue is plainly the binding
// constraint. `--concurrency` overrides it, and the host's core count is in the record, because a row
// whose machine is not written beside it measures nothing.
//
// # The client is what runs out first, and a run says so
//
// A connection costs the client an ephemeral port and a descriptor, and both are exhausted long before a
// server is. Two things follow, and both are here rather than in the probe:
//
// * The probe starts with its soft descriptor limit raised to the hard ceiling, on the platforms that
//   have one. Bun cannot raise its own, so each probe run goes through `sh`'s `ulimit` and the record
//   names the limit that shell reached.
// * **The sweep runs before the in-flight leg, never after.** The in-flight leg opens ten thousand sockets
//   and leaves them in `TIME_WAIT` for minutes; a sweep behind it would be measuring a client that cannot
//   open connections and reporting it as the server slowing down. `--settle` is the wait between the two
//   for a run that wants to be sure.
//
// A run that could not open what it asked for reports `client_limited` and the operating system's own
// error, so a client ceiling is never recorded as the server's. A record's `host` names the machine
// the way `bun nv bench` does.

import { existsSync, statSync } from "node:fs";
import { cpus } from "node:os";
import { basename, dirname, join } from "node:path";
import { rel, ROOT } from "../lib/paths.ts";
import { ArgError, fixed, parseArgs, pyInt, pyRepr } from "../lib/py.ts";
import { Fail, findNvs, freePort, grouped, host, repoRelative, Server, stamp, version, warnIfStale, writeServeRecord } from "./bench.ts";

export const summary = "saturate `nvs serve` on this machine and verify every answer: nv bench-load [--record PATH] [--concurrency N,N]";

const WINDOWS = process.platform === "win32";
const CASE = join(ROOT, "benches", "serve", "echo.nvs");
const PROBE_PACKAGE = "nvs-serve-probe";
const PROBE_BIN = "nvs-serve-probe";

const USAGE =
  "usage: nv bench-load [-h] [--nvs NVS] [--allow-debug] [--probe PROBE] [--concurrency CONCURRENCY]\n" +
  "                     [--seconds SECONDS] [--in-flight IN_FLIGHT] [--rounds ROUNDS] [--settle SETTLE]\n" +
  "                     [--timeout-ms TIMEOUT_MS] [--record RECORD]";

interface Options {
  nvs: string | null;
  allowDebug: boolean;
  probe: string | null;
  concurrency: string | null;
  seconds: number;
  inFlight: number;
  rounds: number;
  settle: number;
  timeoutMs: number;
  record: string | null;
}

/** The options argparse would have read, or null when `--help` asked for the help instead. */
function options(args: string[]): Options | null {
  const p = parseArgs(args, {
    flags: ["--allow-debug"],
    valued: ["--nvs", "--probe", "--concurrency", "--seconds", "--in-flight", "--rounds", "--settle", "--timeout-ms", "--record"],
  });
  if (p.flags.has("--help")) return null;
  const int = (o: string, dflt: number): number => {
    const word = p.values.get(o);
    if (word === undefined) return dflt;
    const n = pyInt(word);
    if (n === null) throw new ArgError(`argument ${o}: invalid int value: ${pyRepr(word)}`);
    return n;
  };
  return {
    nvs: p.values.get("--nvs") ?? null,
    allowDebug: p.flags.has("--allow-debug"),
    probe: p.values.get("--probe") ?? null,
    concurrency: p.values.get("--concurrency") ?? null,
    seconds: int("--seconds", 10),
    inFlight: int("--in-flight", 10_000),
    rounds: int("--rounds", 3),
    settle: int("--settle", 0),
    timeoutMs: int("--timeout-ms", 5_000),
    record: p.values.get("--record") ?? null,
  };
}

function help(): string {
  return [
    USAGE,
    "",
    "Saturate `nvs serve` on this machine and verify every answer.",
    "",
    "options:",
    "  -h, --help            show this help message and exit",
    "  --nvs NVS             path to the nvs binary (default target/release)",
    "  --allow-debug         permit a debug Novis binary",
    "  --probe PROBE         path to a built nvs-serve-probe (default: build it)",
    "  --concurrency N,N     comma-separated connection counts for the sweep (default: derived from the core count)",
    "  --seconds SECONDS     seconds per sweep point (default 10)",
    "  --in-flight N         requests to put in flight at once, or 0 to skip that leg (default 10000)",
    "  --rounds ROUNDS       in-flight rounds (default 3)",
    "  --settle SECONDS      seconds to wait between the sweep and the in-flight leg, for ports to drain",
    "  --timeout-ms MS       a response slower than this is a stall (default 5000)",
    "  --record PATH         append this run to the JSON array at PATH",
  ].join("\n");
}

/**
 * The probe binary, built if it is not already named. Built in release: a debug generator is slower than
 * the server it is driving, which makes every number the generator's own speed rather than the server's.
 */
function probePath(explicit: string | null): string {
  if (explicit !== null) {
    if (!existsSync(explicit) || !statSync(explicit).isFile()) throw new Fail(`--probe ${explicit} is not a file`);
    return explicit;
  }
  const built = join(ROOT, "target", "release", `${PROBE_BIN}${WINDOWS ? ".exe" : ""}`);
  console.log(`  building ${PROBE_PACKAGE} (release)`);
  const done = Bun.spawnSync(["cargo", "build", "--release", "-p", PROBE_PACKAGE], { cwd: ROOT, stdout: "inherit", stderr: "inherit" });
  if (done.exitCode !== 0) throw new Fail(`cargo build -p ${PROBE_PACKAGE} failed with ${done.exitCode}`);
  if (!existsSync(built)) throw new Fail(`${PROBE_PACKAGE} built but ${built} is not there`);
  return built;
}

/**
 * The shell line every probe run starts under: the soft descriptor limit lifted to the hard one. macOS
 * ships a soft limit of 256 and Linux 1024, either of which stops the in-flight leg long before the server
 * does -- and the failure looks exactly like a server that will not accept. Windows has no such limit.
 */
const RAISE = 'ulimit -n "$(ulimit -Hn)" 2>/dev/null';

/** The descriptor limit a probe runs under, or null on a platform without one. */
function descriptorLimit(): string | null {
  if (WINDOWS) return null;
  const out = Bun.spawnSync(["sh", "-c", `${RAISE}; ulimit -n`], { stdout: "pipe", stderr: "pipe" });
  const text = out.stdout.toString().trim();
  return out.exitCode === 0 && text ? text : null;
}

/** One line per connection count from the core count: one, one per core, and out past saturation. */
export function defaultSweep(cores: number): number[] {
  const widths = new Set([1, 2, cores, cores * 2, cores * 8, cores * 32]);
  return [...widths].filter((w) => w > 0 && w <= 2048).sort((a, b) => a - b);
}

type ProbeRecord = Record<string, any>;

/**
 * One probe run, as its record. A probe that could not parse its own arguments is fatal. Spawned rather
 * than run synchronously, so the server's output is drained while the probe drives it.
 */
async function runProbe(probe: string, port: number, mode: string, connections: number, flags: Record<string, number>): Promise<ProbeRecord> {
  const argv = [probe, "--target", `127.0.0.1:${port}`, "--mode", mode, "--connections", String(connections)];
  for (const [name, value] of Object.entries(flags)) argv.push(`--${name}`, String(value));
  const command = WINDOWS ? argv : ["sh", "-c", `${RAISE}; exec "$@"`, "sh", ...argv];
  const proc = Bun.spawn(command, { stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, code] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text(), proc.exited]);
  if (code === 2 || !stdout.trim()) throw new Fail(`${PROBE_BIN} refused the run: ${stderr.trim() || code}`);
  let record: ProbeRecord;
  try {
    record = JSON.parse(stdout);
  } catch (e) {
    throw new Fail(`${PROBE_BIN} did not print a record (${e instanceof Error ? e.message : e}):\n${stdout}\n${stderr}`);
  }
  record.wrong = code === 1;
  return record;
}

const n0 = (x: number) => grouped(x, 0);

/** One line per concurrency, with the verification columns beside the throughput ones. */
function sweepTable(rows: ProbeRecord[]): void {
  console.log();
  console.log(
    `  ${"conns".padStart(6)}  ${"req/s".padStart(10)}  ${"p50 us".padStart(8)}  ${"p99 us".padStart(8)}  ${"max us".padStart(10)}  ` +
      `${"verified".padStart(10)}  ${"wrong".padStart(6)}  ${"stalls".padStart(7)}  note`,
  );
  for (const row of rows) {
    const wrong = row.bad_status + row.bad_body + row.leftover;
    const note = row.client_limited ? `client-limited at ${row.connections_opened}` : "";
    console.log(
      `  ${String(row.connections_requested).padStart(6)}  ${n0(row.requests_per_sec).padStart(10)}  ` +
        `${n0(row.p50_us).padStart(8)}  ${n0(row.p99_us).padStart(8)}  ${n0(row.max_us).padStart(10)}  ` +
        `${n0(row.verified).padStart(10)}  ${n0(wrong).padStart(6)}  ${n0(row.stalls).padStart(7)}  ${note}`,
    );
  }
}

/** One line per round of the simultaneity leg. */
function inFlightTable(record: ProbeRecord): void {
  console.log();
  if (record.client_limited) {
    console.log(`  in flight: opened ${n0(record.connections_opened)} of ${n0(record.connections_requested)} -- ${record.connect_error}`);
    console.log("             the ceiling below is this client's, not the server's");
  }
  for (const row of record.rounds) {
    console.log(
      `  round ${row.round}: in_flight=${n0(row.in_flight)} verified=${n0(row.verified)} mismatched=${n0(row.mismatched)} ` +
        `broken=${n0(row.broken)} unanswered=${n0(row.unanswered)} all_answered_in=${fixed(row.all_answered_in_s, 3)}s`,
    );
  }
}

async function main(o: Options): Promise<number> {
  if (!existsSync(CASE)) throw new Fail(`${CASE} is missing; this leg's case is part of the tree`);
  const binary = findNvs(o.nvs, o.allowDebug);
  warnIfStale(binary);
  const probe = probePath(o.probe);
  const descriptors = descriptorLimit();
  const cores = cpus().length || 1;
  const widths =
    o.concurrency !== null
      ? o.concurrency
          .split(",")
          .filter((w) => w.trim())
          .map((w) => {
            const n = pyInt(w);
            if (n === null) throw new Fail(`--concurrency: ${pyRepr(w)} is not a whole number`);
            return n;
          })
      : defaultSweep(cores);

  const port = await freePort();
  console.log(`  serving ${rel(CASE)} on 127.0.0.1:${port} (${cores} cores${descriptors ? `, ${descriptors} descriptors)` : ")"}`);
  const server = await Server.start([binary, "serve", basename(CASE), "--port", String(port)], port, dirname(CASE));
  const rows: ProbeRecord[] = [];
  let flight: ProbeRecord | null = null;
  try {
    for (const width of widths) {
      console.log(`  closed loop: ${width} connection(s) for ${o.seconds}s`);
      rows.push(await runProbe(probe, port, "closed-loop", width, { seconds: o.seconds, "timeout-ms": o.timeoutMs }));
    }
    if (o.inFlight > 0) {
      if (o.settle) {
        console.log(`  settling ${o.settle}s before the in-flight leg`);
        await Bun.sleep(o.settle * 1000);
      }
      console.log(`  in flight: ${o.inFlight} at once, ${o.rounds} round(s)`);
      flight = await runProbe(probe, port, "in-flight", o.inFlight, { rounds: o.rounds, "timeout-ms": o.timeoutMs });
    }
  } finally {
    await server.stop();
  }

  sweepTable(rows);
  if (flight) inFlightTable(flight);

  let wrong = rows.reduce((sum, row) => sum + row.bad_status + row.bad_body + row.leftover, 0);
  if (flight) wrong += flight.rounds.reduce((sum: number, row: ProbeRecord) => sum + row.mismatched + row.broken + row.unanswered, 0);
  const notes: string[] = [...rows.flatMap((row) => row.notes), ...(flight ? flight.notes : [])];
  console.log();
  if (wrong) {
    console.log(`  FAIL: ${n0(wrong)} answer(s) did not belong to the request that asked`);
    for (const note of notes.slice(0, 10)) console.log(`    ! ${note}`);
  } else {
    const best = Math.max(0, ...rows.map((row) => row.requests_per_sec));
    const served = rows.reduce((sum, row) => sum + row.verified, 0);
    console.log(`  every one of ${n0(served)} answers belonged to the request that asked`);
    console.log(`  peak ${n0(best)} req/s across ${rows.length} concurrency point(s)`);
  }

  const record = {
    date: stamp(),
    commit: version("git", ["rev-parse", "--short", "HEAD"]),
    case: basename(CASE),
    seconds_per_point: o.seconds,
    nvs_version: version(binary, ["--version"]),
    nvs_binary: repoRelative(binary),
    sweep: rows,
    in_flight: flight,
    wrong,
    host: { ...host(), descriptor_limit: descriptors },
  };
  if (o.record) writeServeRecord(o.record, record);
  else console.log("  not recorded: pass --record PATH to append this run to a JSON array");
  return wrong ? 1 : 0;
}

export async function run(args: string[]): Promise<number> {
  let o: Options | null;
  try {
    o = options(args);
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv bench-load: error: ${e.message}`);
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
