// `bun nv valgrind`: every program under `examples/` runs under valgrind's memcheck, and a definite leak or
// an invalid memory access in any of them is red.
//
//     bun nv valgrind --nvs <binary> [<file>...]
//
// It runs on Linux, where valgrind is. CI's nightly deep lane runs it in the job `valgrind`
// (`rule:testing/the-deep-lane`), and it is the one run that looks for leaks from end to end: CI's `asan`
// job runs with leak detection off, and `tools/leak-check.sh` runs only the files a person names. It
// builds nothing and has no default binary, so it never runs a stale build it happened to find: `--nvs`
// names the build, and a missing one is a usage error.
//
// The files are the ones named, else every `examples/*.nvs`. Each runs as `nvs run <file>` from the
// repository root, as `crates/nvs-cli/tests/examples.rs` runs it: a `.nvsr` beside it is the request it
// answers (`--request`), and a `.toml` beside it is its configuration in place of `nvs.toml`. The binary
// is in `NVS_BIN` for a program that starts `nvs` itself. An example that needs a cache server, a
// database or a queue table `nvs queue migrate` has not made stops at that error, and its error path is
// what memcheck reads.
//
// `examples/limits.nvs` runs with `tools/valgrind-limits.toml` layered over `nvs.toml`, because its cost
// under memcheck is the distance to a memory ceiling its leak verdict does not depend on. Naming any
// `--config` turns off the search for `./nvs.toml`, so the repository's own file is named first. Only exit
// 97 is a leak: a program's own exit status passes through valgrind, and one that ends in an error by
// design must not read as one. A program that did not finish is red too, since nothing was checked, and
// so is a machine with no valgrind.
//
// `examples/http.nvs` talks to `http://127.0.0.1:8099`, so the run holds the local origin
// (`accept/origin.ts`) for its whole length. Several programs run at once, as wide as `lib/machine.ts`
// decides for this machine (`NVS_VALGRIND_JOBS` overrides it). It never stops early: every red program is
// named, in file order. What it spends: on a 16-core machine, 8 at a time, the 82 examples took about
// two minutes over the debug build.
//
// Every process it starts goes through `ValgrindSeams`, so `test/valgrind.test.ts` drives it with
// recorded outcomes and no valgrind.
//
// It exits 0 when every program is clean, 1 when one is not or valgrind is missing, and 2 on a usage error
// or off Linux.

import { existsSync, readdirSync } from "node:fs";
import { isAbsolute, join } from "node:path";
import { firstErrLine, type Outcome } from "../accept/accept.ts";
import { holdOrigin, type Origin } from "../accept/origin.ts";
import * as machine from "../lib/machine.ts";
import { ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";

export const summary = "every examples/ program under valgrind's memcheck, red on a definite leak or an invalid access: nv valgrind --nvs <binary> [<file>...]";

const USAGE = "usage: bun nv valgrind --nvs <binary> [<file>...]";

/** The exit status valgrind is told to use for an error it found, apart from any a program uses itself. */
export const VALGRIND_ERROR = 97;

/** A program that runs under a configuration layered over `nvs.toml`, and which. */
export const VALGRIND_CONFIG: Record<string, string> = { "examples/limits.nvs": "tools/valgrind-limits.toml" };

/** One program's run: a bound far over what any example takes under memcheck. */
const TIMEOUT_MS = 30 * 60 * 1000;

/** Everything the command reads or starts, replaced under test. Each one left out is the real one. */
export interface ValgrindSeams {
  platform: NodeJS.Platform;
  hasValgrind(): boolean;
  exists(path: string): boolean;
  /** The `.nvs` files directly under `examples/`, as repository paths. */
  examples(): string[];
  /** Runs `argv` from the repository root with `env` added. A program that did not finish is exit -1. */
  run(argv: string[], env: Record<string, string>): Promise<Outcome>;
  origin(): Promise<Origin>;
  /** How many programs run at once, for `ceiling` programs. */
  jobs(ceiling: number): number;
  say(line: string): void;
}

/** What sits beside a program: its own configuration and the request it answers, as repository paths. */
export interface Beside {
  toml?: string;
  nvsr?: string;
}

/** The files beside `file` that change how it runs. */
export function besideOf(file: string, exists: (path: string) => boolean): Beside {
  const stem = file.replace(/\.nvs$/, "");
  const out: Beside = {};
  if (exists(`${stem}.toml`)) out.toml = `${stem}.toml`;
  if (exists(`${stem}.nvsr`)) out.nvsr = `${stem}.nvsr`;
  return out;
}

/** The valgrind command for one program, run from the repository root. */
export function valgrindArgv(binary: string, file: string, beside: Beside = {}): string[] {
  const over = VALGRIND_CONFIG[file];
  const configs = beside.toml !== undefined ? [beside.toml] : over !== undefined ? ["nvs.toml", over] : [];
  return [
    "valgrind",
    `--error-exitcode=${VALGRIND_ERROR}`,
    "--leak-check=full",
    "--errors-for-leak-kinds=definite",
    "--suppressions=tools/valgrind.supp",
    "-q",
    binary,
    ...configs.flatMap((c) => ["--config", c]),
    "run",
    ...(beside.nvsr !== undefined ? ["--request", beside.nvsr] : []),
    file,
  ];
}

/** The red line for one program's outcome, or "" when valgrind found nothing. */
export function verdict(file: string, r: Outcome): string {
  if (r.code === VALGRIND_ERROR) return `valgrind ${file}: exit ${r.code} -- ${firstErrLine(r)}`;
  if (r.code === -1) return `valgrind ${file}: did not finish -- ${firstErrLine(r)}`;
  return "";
}


async function capture(argv: string[], env: Record<string, string>): Promise<Outcome> {
  try {
    const r = await proc(argv, { cwd: ROOT, env, timeoutMs: TIMEOUT_MS });
    return { code: r.timedOut ? -1 : r.code, out: r.stdout, err: r.timedOut ? `timed out after ${TIMEOUT_MS / 1000}s` : r.stderr };
  } catch (e) {
    return { code: -1, out: "", err: String((e as Error).message ?? e) };
  }
}

const REAL: ValgrindSeams = {
  platform: process.platform,
  hasValgrind: () => Bun.which("valgrind") !== null,
  exists: (path) => existsSync(isAbsolute(path) ? path : join(ROOT, path)),
  examples: () =>
    readdirSync(join(ROOT, "examples"))
      .filter((f) => f.endsWith(".nvs"))
      .map((f) => `examples/${f}`),
  run: capture,
  origin: () => holdOrigin(),
  jobs: (ceiling) => machine.jobs("local", { ceiling, envs: ["NVS_VALGRIND_JOBS"] }),
  say: (line) => console.log(line),
};

/** Runs `files` under valgrind over `binary`, `jobs` at a time: every red line, in the order of `files`. */
export async function sweep(binary: string, files: string[], s: ValgrindSeams): Promise<string[]> {
  const width = Math.max(1, Math.min(s.jobs(files.length), files.length));
  s.say(`valgrind: ${files.length} program(s), ${width} at a time, over ${binary}`);
  const origin = await s.origin();
  s.say(origin.line);
  const reds = new Map<string, string>();
  try {
    let next = 0;
    const worker = async () => {
      while (next < files.length) {
        const f = files[next++]!;
        const red = verdict(f, await s.run(valgrindArgv(binary, f, besideOf(f, s.exists)), { NVS_BIN: binary }));
        s.say(red === "" ? `  ok   ${f}` : `  RED  ${red}`);
        if (red !== "") reds.set(f, red);
      }
    };
    await Promise.all(Array.from({ length: width }, worker));
  } finally {
    origin.close();
  }
  return files.flatMap((f) => (reds.has(f) ? [reds.get(f)!] : []));
}

/** Parses, runs and prints; returns the exit code. */
export async function valgrind(args: string[], over: Partial<ValgrindSeams> = {}): Promise<number> {
  const s: ValgrindSeams = { ...REAL, ...over };
  let binary = "";
  const named: string[] = [];
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--nvs" && args[i + 1] !== undefined) binary = args[++i]!;
    else if (a.startsWith("-")) {
      console.error(`${USAGE}\nnv valgrind: error: ${a === "--nvs" ? "--nvs needs a path" : `unexpected argument ${a}`}`);
      return 2;
    } else named.push(a.replaceAll("\\", "/"));
  }
  if (s.platform !== "linux") {
    console.error(`nv valgrind: error: valgrind runs on Linux only. On Windows, run tools/leak-check.sh inside WSL.`);
    return 2;
  }
  if (binary === "") {
    console.error(`${USAGE}\nnv valgrind: error: --nvs names the binary to check`);
    return 2;
  }
  if (!s.exists(binary)) {
    console.error(`${USAGE}\nnv valgrind: error: no binary at ${binary}`);
    return 2;
  }
  const missing = named.filter((f) => !s.exists(f));
  if (missing.length > 0) {
    console.error(`${USAGE}\nnv valgrind: error: no such file: ${missing.join(", ")}`);
    return 2;
  }
  if (!s.hasValgrind()) {
    s.say("RED valgrind is not on PATH, so nothing was checked");
    return 1;
  }
  const files = named.length > 0 ? named : s.examples().sort();
  const reds = await sweep(binary, files, s);
  if (reds.length > 0) {
    s.say(`RED ${reds.length} of ${files.length} program(s):`);
    for (const r of reds) s.say(`  ${r}`);
    return 1;
  }
  s.say(`GREEN ${files.length} program(s) under valgrind, no definite leak and no invalid access`);
  return 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return valgrind(args);
}
