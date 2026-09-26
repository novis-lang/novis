// Running another program. An argument list goes to the program as it is, never through a shell, so
// no quoting rule of any shell applies to it. Every run has a timeout, and a program that outlives it
// is killed and reported as timed out.

import { ROOT } from "./paths.ts";
import { erase } from "./progress.ts";

export interface RunOptions {
  cwd?: string;
  /** Milliseconds before the program is killed. */
  timeoutMs?: number;
  env?: Record<string, string>;
  /** Text written to the program's standard input. */
  input?: string;
  /** Called with each line the program prints, as it prints it, without its line ending. */
  onLine?: (line: string, stream: "stdout" | "stderr") => void;
}

export interface RunResult {
  argv: string[];
  code: number;
  stdout: string;
  stderr: string;
  timedOut: boolean;
}

const DEFAULT_TIMEOUT_MS = 10 * 60 * 1000;

/**
 * `argv` with a bare program name replaced by its path on `PATH`. On Windows, `Bun.spawn` handed an
 * `env` finds only an `.exe` by a bare name, so `npm`, which is `npm.cmd`, fails to start unless
 * `Bun.which` resolves it first, the way a shell would through `PATHEXT`. Windows starts a `.cmd`
 * under `cmd.exe`, so every argument handed to one is one this tree wrote.
 */
export function resolved(argv: string[], env: Record<string, string | undefined>): string[] {
  const exe = argv[0]!;
  if (process.platform !== "win32" || /[\\/]/.test(exe)) return argv;
  const found = Bun.which(exe, { PATH: env.PATH ?? env.Path ?? "" });
  return found === null ? argv : [found, ...argv.slice(1)];
}

/** Runs `argv` to completion and returns what it printed. A program that cannot be started throws. */
export async function run(argv: string[], opts: RunOptions = {}): Promise<RunResult> {
  if (argv.length === 0) throw new Error("proc.run: an empty argument list");
  const env = opts.env ? { ...process.env, ...opts.env } : process.env;
  const child = Bun.spawn(resolved(argv, env), {
    cwd: opts.cwd ?? ROOT,
    env,
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
      opts.onLine ? drain(child.stdout, (l) => opts.onLine!(l, "stdout")) : new Response(child.stdout).text(),
      opts.onLine ? drain(child.stderr, (l) => opts.onLine!(l, "stderr")) : new Response(child.stderr).text(),
      child.exited,
    ]);
    return { argv, code: timedOut ? 124 : code, stdout, stderr, timedOut };
  } finally {
    clearTimeout(timer);
  }
}

/** All of `stream` as text, with `onLine` called on each line as it arrives. A line ends at LF, and a
 * CR before it is dropped; a last line with no LF is passed as well. */
async function drain(stream: ReadableStream<Uint8Array>, onLine: (line: string) => void): Promise<string> {
  const decoder = new TextDecoder();
  let all = "";
  let pending = "";
  for await (const chunk of stream) {
    const text = decoder.decode(chunk, { stream: true });
    all += text;
    pending += text;
    let nl = pending.indexOf("\n");
    while (nl >= 0) {
      onLine(pending.slice(0, nl).replace(/\r$/, ""));
      pending = pending.slice(nl + 1);
      nl = pending.indexOf("\n");
    }
  }
  const tail = decoder.decode();
  all += tail;
  pending += tail;
  if (pending) onLine(pending.replace(/\r$/, ""));
  return all;
}

/** Runs `argv` with its output going straight to this process's own, and returns its exit status. The
 * progress line, when one is shown, is erased first. */
export async function passthrough(argv: string[], opts: Omit<RunOptions, "input" | "onLine"> = {}): Promise<number> {
  erase();
  const env = opts.env ? { ...process.env, ...opts.env } : process.env;
  const child = Bun.spawn(resolved(argv, env), {
    cwd: opts.cwd ?? ROOT,
    env,
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
