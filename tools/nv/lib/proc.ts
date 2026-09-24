// Running another program. An argument list goes to the program as it is, never through a shell, so
// no quoting rule of any shell applies to it. Every run has a timeout, and a program that outlives it
// is killed and reported as timed out.

import { ROOT } from "./paths.ts";

export interface RunOptions {
  cwd?: string;
  /** Milliseconds before the program is killed. */
  timeoutMs?: number;
  env?: Record<string, string>;
  /** Text written to the program's standard input. */
  input?: string;
}

export interface RunResult {
  argv: string[];
  code: number;
  stdout: string;
  stderr: string;
  timedOut: boolean;
}

const DEFAULT_TIMEOUT_MS = 10 * 60 * 1000;

/** Runs `argv` to completion and returns what it printed. A program that cannot be started throws. */
export async function run(argv: string[], opts: RunOptions = {}): Promise<RunResult> {
  if (argv.length === 0) throw new Error("proc.run: an empty argument list");
  const child = Bun.spawn(argv, {
    cwd: opts.cwd ?? ROOT,
    env: opts.env ? { ...process.env, ...opts.env } : process.env,
    stdin: opts.input === undefined ? "ignore" : new TextEncoder().encode(opts.input),
    stdout: "pipe",
    stderr: "pipe",
  });
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    child.kill();
  }, opts.timeoutMs ?? DEFAULT_TIMEOUT_MS);
  try {
    const [stdout, stderr, code] = await Promise.all([
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
      child.exited,
    ]);
    return { argv, code: timedOut ? 124 : code, stdout, stderr, timedOut };
  } finally {
    clearTimeout(timer);
  }
}

/** Runs `argv` with its output going straight to this process's own, and returns its exit status. */
export async function passthrough(argv: string[], opts: Omit<RunOptions, "input"> = {}): Promise<number> {
  const child = Bun.spawn(argv, {
    cwd: opts.cwd ?? ROOT,
    env: opts.env ? { ...process.env, ...opts.env } : process.env,
    stdin: "ignore",
    stdout: "inherit",
    stderr: "inherit",
  });
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    child.kill();
  }, opts.timeoutMs ?? DEFAULT_TIMEOUT_MS);
  try {
    const code = await child.exited;
    return timedOut ? 124 : code;
  } finally {
    clearTimeout(timer);
  }
}
