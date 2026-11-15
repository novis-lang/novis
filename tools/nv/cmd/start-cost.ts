// `bun nv start-cost`: what one start of `nvs` costs, timed by command and counted by phase.
//
//     bun nv start-cost             25 runs of each command on each data folder
//     bun nv start-cost --runs 5    fewer runs, for a quick look
//
// The timings run `target/release/nvs` on a one-line program: `--version`, `ast`, `check` and `run`, each
// on a shared data folder that earlier runs have warmed and on a fresh one made for that run alone. The
// runs interleave, one of each shape in turn, so a burst of load lands on every shape alike. It prints
// the median and the spread of each. Then it runs `target/debug/nvs check --count` and `run --count` once
// each and prints their `phases:` lines as a table: each start phase's allocations, bytes and file work
// (`crates/nvs-cli/src/phases.rs`). The counts are the figures that decide, since they are the same on
// every machine; the times are a note (`rule:testing/perf-two-mechanisms`).
//
// Everything it writes is under `.agent-tmp/start-cost/`, and each fresh data folder is deleted after
// its run. Build both binaries first: `bun nv relink --build` and `cargo build`.
//
// What it spends: about a minute at 25 runs, on one core at a time.

import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT, rel } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { releaseCli } from "../lib/relink.ts";

export const summary = "time one start of nvs per command and count its work per start phase: nv start-cost [--runs N]";

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
}

/** The command's arguments, or the message that says what is wrong with them. */
export function parse(args: string[]): Args | string {
  let runs = 25;
  for (let i = 0; i < args.length; i++) {
    const arg = args[i]!;
    if (arg === "--runs") {
      const next = args[++i];
      if (next === undefined) return "--runs needs a number";
      const n = Number(next);
      if (!Number.isInteger(n) || n < 1) return `--runs needs a whole number of 1 or more, not ${next}`;
      runs = n;
    } else return `unknown argument ${arg}`;
  }
  return { runs };
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
  const release = releaseCli();
  const debug = join(ROOT, "target", "debug", process.platform === "win32" ? "nvs.exe" : "nvs");
  for (const [exe, build] of [[release, "bun nv relink --build"], [debug, "cargo build"]] as const) {
    if (!existsSync(exe)) {
      console.error(`nv start-cost: ${rel(exe)} does not exist; run \`${build}\` first`);
      return 1;
    }
  }
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
