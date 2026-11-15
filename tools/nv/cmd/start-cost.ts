// `bun nv start-cost`: what one start of `nvs` costs, timed by command and counted by phase.
//
//     bun nv start-cost             25 runs of each command on each data folder
//     bun nv start-cost --runs 5    fewer runs, for a quick look
//
// The timings run `target/release/nvs` on a one-line program: `--version`, `ast`, `check` and `run`, each
// on a shared data folder that earlier runs have warmed and on a fresh one made for that run alone. The
// runs interleave, one of each shape in turn, so a burst of load lands on every shape alike. It prints
// the median and the spread of each. Then it builds the pipeline's debug `nvs` (`lib/covws.ts`), runs its
// `check --count` and `run --count` once each, and prints their `phases:` lines as a table: each start phase's allocations, bytes and file work
// (`crates/nvs-cli/src/phases.rs`). The counts are the figures that decide, since they are the same on
// every machine; the times are a note (`rule:testing/perf-two-mechanisms`).
//
// Everything it writes is under `.agent-tmp/start-cost/`, and each fresh data folder is deleted after
// its run. Build the release binary first with `bun nv relink --build`.
//
//     bun nv start-cost --floor     what a start costs before `main` runs, on Linux in WSL
//
// `--floor` measures the part of a start the binary's size costs. It syncs the WSL copy of the tree
// (`accept/mirror.ts`), builds a release `nvs` and `nvs-serve-probe` there into `linux-test`'s target
// directory, and runs `nvs --version` and `nvs-serve-probe --help` under `/usr/bin/time`, interleaved.
// The probe is the smallest release binary of the workspace that links `std`, and it has no `--version`,
// so its clap help is the comparison: each parses its arguments, prints a few lines and exits. It
// prints each binary's size and the median clock, minor and major page faults of each. The faults are
// counted by the kernel, so they are the figures that decide; the clock is a note. Windows only, like
// `linux-test`.
//
// What it spends: about a minute at 25 runs, on one core at a time. `--floor`'s first build in a cold
// target directory takes tens of minutes; each one after it relinks the crates that changed.

import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { mirrorPath, q, syncMirror, wslPath } from "../accept/mirror.ts";
import { buildCovws } from "../lib/covws.ts";
import { ROOT, rel } from "../lib/paths.ts";
import { passthrough, run as runProc } from "../lib/proc.ts";
import { releaseCli } from "../lib/relink.ts";
import { wslArgv } from "../lib/wsl.ts";

export const summary =
  "time one start of nvs per command and count its work per start phase: nv start-cost [--runs N] [--floor]";

/** `linux-test`'s target directory in the distro, so `--floor` reuses what its builds compiled. */
const WSL_TARGET = "/var/tmp/nvs-target-wsl";
/** The two binaries `--floor` compares, each with the arguments it runs. */
export const FLOOR: [string, string[]][] = [
  ["nvs", ["--version"]],
  ["nvs-serve-probe", ["--help"]],
];

const WORK = join(ROOT, ".agent-tmp", "start-cost");
const PROGRAM = '<?nvs\necho "hi\\n";\n';
const COMMANDS: [string, string[]][] = [
  ["--version", ["--version"]],
  ["ast", ["ast", "small.nvs"]],
  ["check", ["check", "small.nvs"]],
  ["run", ["run", "small.nvs"]],
];

export interface Args {
  runs: number;
  floor: boolean;
}

/** The command's arguments, or the message that says what is wrong with them. */
export function parse(args: string[]): Args | string {
  let runs = 25;
  let floor = false;
  for (let i = 0; i < args.length; i++) {
    const arg = args[i]!;
    if (arg === "--runs") {
      const next = args[++i];
      if (next === undefined) return "--runs needs a number";
      const n = Number(next);
      if (!Number.isInteger(n) || n < 1) return `--runs needs a whole number of 1 or more, not ${next}`;
      runs = n;
    } else if (arg === "--floor") floor = true;
    else return `unknown argument ${arg}`;
  }
  return { runs, floor };
}

/**
 * The `bash` line that measures the floor in the distro. It prints `size <name> <bytes>` once per
 * binary, then `run <name> <ns> <minor> <major>` per run, `runs` rounds with one run of each binary in
 * turn. It runs from `/var/tmp`, a directory with no `nvs.toml`.
 */
export function floorLine(release: string, runs: number): string {
  const exe = (name: string) => q(`${release}/${name}`);
  const sizes = FLOOR.map(([name]) => `echo size ${name} $(stat -c %s ${exe(name)})`).join(" && ");
  const once = FLOOR.map(
    ([name, argv]) =>
      `s=$(date +%s%N); /usr/bin/time -f '%R %F' -o "$t" ${exe(name)} ${argv.map(q).join(" ")} >/dev/null || exit 1; ` +
      `e=$(date +%s%N); echo run ${name} $((e - s)) $(cat "$t")`,
  ).join("; ");
  return `set -e; cd /var/tmp && t=$(mktemp) && ${sizes} && for i in $(seq ${runs}); do ${once}; done; rm -f "$t"`;
}

/** One binary's figures from `floorLine`'s output: its size, and per run the clock and page faults. */
export interface Floor {
  bytes: number;
  ms: number[];
  minor: number[];
  major: number[];
}

/** `floorLine`'s output read by binary. A line it does not print is skipped. */
export function readFloor(stdout: string): Map<string, Floor> {
  const out = new Map<string, Floor>();
  const of = (name: string) => {
    const row = out.get(name) ?? { bytes: NaN, ms: [], minor: [], major: [] };
    out.set(name, row);
    return row;
  };
  for (const line of stdout.split(/\r?\n/)) {
    const size = /^size (\S+) (\d+)$/.exec(line);
    if (size !== null) of(size[1]!).bytes = Number(size[2]);
    const r = /^run (\S+) (\d+) (\d+) (\d+)$/.exec(line);
    if (r === null) continue;
    const row = of(r[1]!);
    row.ms.push(Number(r[2]) / 1e6);
    row.minor.push(Number(r[3]));
    row.major.push(Number(r[4]));
  }
  return out;
}

/** `--floor`: build both binaries in WSL, run each `runs` times, and print the medians. */
async function floor(runs: number): Promise<number> {
  if (process.platform !== "win32") {
    console.error("nv start-cost: --floor runs in WSL and this host is not Windows");
    return 2;
  }
  const mirror = mirrorPath(WSL_TARGET, wslPath(ROOT));
  console.log(`start-cost: syncing the WSL copy of the tree at ${mirror}`);
  const synced = await syncMirror(mirror);
  if (synced !== "") {
    console.error(`nv start-cost: the WSL copy of the tree could not be synced: ${synced}`);
    return 2;
  }
  const bins = FLOOR.map(([name]) => `--bin ${name}`).join(" ");
  const build = `cd ${q(mirror)} && CARGO_TARGET_DIR=${q(WSL_TARGET)} cargo build --release ${bins}`;
  // cargo prints nothing while thin LTO links one unit, so the watch is off and the timeout bounds it.
  const built = await passthrough(wslArgv(build), { timeoutMs: 2 * 60 * 60 * 1000, hang: false });
  if (built !== 0) return built;
  const result = await runProc(wslArgv(floorLine(`${WSL_TARGET}/release`, runs)), { timeoutMs: 10 * 60 * 1000 });
  const table = readFloor(result.stdout);
  if (result.code !== 0 || table.size !== FLOOR.length) {
    console.error(`nv start-cost: the floor run exited ${result.code}`);
    console.error(result.stderr.trimEnd().split("\n").slice(-10).join("\n"));
    return 1;
  }
  console.log(`\nLinux release start floor, ${runs} interleaved runs each (median):`);
  for (const [name, argv] of FLOOR) {
    const row = table.get(name)!;
    console.log(
      `  ${`${name} ${argv.join(" ")}`.padEnd(28)} ${(row.bytes / 1e6).toFixed(1).padStart(6)} MB` +
        `  ${median(row.ms).toFixed(2).padStart(6)} ms  minor ${median(row.minor)}  major ${median(row.major)}`,
    );
  }
  return 0;
}

/** The middle value of `values`, or the mean of the two middle values. */
export function median(values: number[]): number {
  if (values.length === 0) return NaN;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = sorted.length >> 1;
  return sorted.length % 2 === 1 ? sorted[mid]! : (sorted[mid - 1]! + sorted[mid]!) / 2;
}

/** The figures of a `phases:` line, by phase and then by figure: `{ core: { allocations: 16624, ... } }`. */
export function phases(stderr: string): Map<string, Map<string, number>> {
  const table = new Map<string, Map<string, number>>();
  const line = stderr.split(/\r?\n/).find((l) => l.startsWith("phases: "));
  if (line === undefined) return table;
  for (const pair of line.slice("phases: ".length).split(" ")) {
    const m = /^([a-z_]+)\.([a-z_]+)=(\d+)$/.exec(pair);
    if (m === null) continue;
    const row = table.get(m[1]!) ?? new Map<string, number>();
    row.set(m[2]!, Number(m[3]));
    table.set(m[1]!, row);
  }
  return table;
}

/** `phases` as aligned text rows, with each phase's share of the allocations. */
export function render(table: Map<string, Map<string, number>>): string[] {
  const figures = ["allocations", "bytes", "read", "settings", "listed", "resolved"];
  const total = [...table.values()].reduce((sum, row) => sum + (row.get("allocations") ?? 0), 0);
  const rows = [["phase", ...figures, "share"]];
  for (const [phase, row] of table) {
    const share = total === 0 ? 0 : ((row.get("allocations") ?? 0) / total) * 100;
    rows.push([phase, ...figures.map((f) => String(row.get(f) ?? 0)), `${share.toFixed(1)}%`]);
  }
  const widths = rows[0]!.map((_, i) => Math.max(...rows.map((r) => r[i]!.length)));
  return rows.map((r) => r.map((cell, i) => (i === 0 ? cell.padEnd(widths[i]!) : cell.padStart(widths[i]!))).join("  "));
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const parsed = parse(args);
  if (typeof parsed === "string") {
    console.error(`nv start-cost: ${parsed}`);
    return 2;
  }
  if (parsed.floor) return floor(parsed.runs);
  const release = releaseCli();
  if (!existsSync(release)) {
    console.error(`nv start-cost: ${rel(release)} does not exist; run \`bun nv relink --build\` first`);
    return 1;
  }
  const debug = (await buildCovws()).nvs;
  mkdirSync(WORK, { recursive: true });
  writeFileSync(join(WORK, "small.nvs"), PROGRAM);
  const shared = join(WORK, "shared");
  mkdirSync(shared, { recursive: true });

  const once = async (exe: string, data: string, argv: string[]) => {
    const started = performance.now();
    const result = await runProc([exe, "--data", data, ...argv], { cwd: WORK, timeoutMs: 60_000 });
    return { ms: performance.now() - started, result };
  };

  // One warm-up of each command on the shared folder, so its first timed run is warm.
  for (const [, argv] of COMMANDS) await once(release, shared, argv);
  const times = new Map<string, number[]>();
  for (let i = 0; i < parsed.runs; i++) {
    for (const [name, argv] of COMMANDS) {
      for (const folder of ["shared", "fresh"] as const) {
        const data = folder === "shared" ? shared : join(WORK, `fresh-${i}`);
        const { ms, result } = await once(release, data, argv);
        if (folder === "fresh") rmSync(data, { recursive: true, force: true });
        if (result.code !== 0) {
          console.error(`nv start-cost: \`nvs ${argv.join(" ")}\` exited ${result.code}`);
          console.error(result.stderr.trimEnd().split("\n").slice(-10).join("\n"));
          return 1;
        }
        const key = `${name} ${folder}`;
        times.set(key, [...(times.get(key) ?? []), ms]);
      }
    }
  }
  console.log(`release timings, ${parsed.runs} runs each (median, fastest to slowest):`);
  for (const [key, ms] of times) {
    const sorted = [...ms].sort((a, b) => a - b);
    console.log(`  ${key.padEnd(18)} ${median(ms).toFixed(1).padStart(7)} ms  (${sorted[0]!.toFixed(1)} to ${sorted.at(-1)!.toFixed(1)})`);
  }

  for (const command of ["check", "run"]) {
    const data = join(WORK, "fresh-count");
    const { result } = await once(debug, data, [command, "--count", "small.nvs"]);
    rmSync(data, { recursive: true, force: true });
    const table = phases(result.stderr);
    if (table.size === 0) {
      console.error(`nv start-cost: the debug \`nvs ${command} --count\` printed no \`phases:\` line`);
      console.error(result.stderr.trimEnd());
      return 1;
    }
    console.log(`\ndebug \`nvs ${command} --count\`, counted by start phase:`);
    for (const line of render(table)) console.log(`  ${line}`);
  }
  return 0;
}
