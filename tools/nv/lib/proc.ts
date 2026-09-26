// Running another program. An argument list goes to the program as it is, never through a shell, so
// no quoting rule of any shell applies to it. Every run has a timeout, and a program that outlives it
// is killed with every process it started, and reported as timed out.
//
// A run that asks for `reap` also has the processes killed that its program started and left behind
// when it failed: a test binary that panics beside the server it spawned would otherwise leave an
// `nvs` holding the binary the next build has to replace. On Windows a process keeps its creator's id
// after the creator exits, so one snapshot of the process table finds them; elsewhere an orphan is
// adopted by init and only a timeout's kill, taken while the tree is whole, reaches it.

import { dlopen, FFIType, ptr } from "bun:ffi";
import { DISCARD_PROFILE, ROOT } from "./paths.ts";
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
  /** When the program exits with a failure, kill every process it started that is still running. */
  reap?: boolean;
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

/**
 * The environment a started program gets: this process's, then `extra`. `LLVM_PROFILE_FILE` is
 * `DISCARD_PROFILE` when neither sets it, so an instrumented binary that no run records writes its
 * counters under `.agent-tmp/`, never a `default_*.profraw` into its working directory.
 */
export function childEnv(extra?: Record<string, string>): Record<string, string | undefined> {
  return { LLVM_PROFILE_FILE: DISCARD_PROFILE, ...process.env, ...extra };
}

/** Runs `argv` to completion and returns what it printed. A program that cannot be started throws. */
export async function run(argv: string[], opts: RunOptions = {}): Promise<RunResult> {
  if (argv.length === 0) throw new Error("proc.run: an empty argument list");
  const env = childEnv(opts.env);
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
    killTree(child.pid);
  }, opts.timeoutMs ?? DEFAULT_TIMEOUT_MS);
  try {
    // The exit status first: a process the program left running holds the pipes open, and reaping it is
    // what lets them close.
    const reading = Promise.all([
      opts.onLine ? drain(child.stdout, (l) => opts.onLine!(l, "stdout")) : new Response(child.stdout).text(),
      opts.onLine ? drain(child.stderr, (l) => opts.onLine!(l, "stderr")) : new Response(child.stderr).text(),
    ]);
    const code = await child.exited;
    if (opts.reap && (code !== 0 || timedOut)) reapOrphans(child.pid);
    const [stdout, stderr] = await reading;
    return { argv, code: timedOut ? 124 : code, stdout, stderr, timedOut };
  } finally {
    clearTimeout(timer);
  }
}

// ---- process trees ---------------------------------------------------------------------------------

/** Each process's id and its creator's id, from one snapshot of the process table. */
export function processTable(): [pid: number, parent: number][] {
  if (process.platform === "win32") {
    try {
      return windowsTable();
    } catch {
      const r = Bun.spawnSync(["powershell", "-NoProfile", "-Command", "Get-CimInstance Win32_Process | ForEach-Object { \"$($_.ProcessId) $($_.ParentProcessId)\" }"], { stdout: "pipe", stderr: "ignore" });
      return parseTable(r.stdout.toString());
    }
  }
  const r = Bun.spawnSync(["ps", "-A", "-o", "pid=,ppid="], { stdout: "pipe", stderr: "ignore" });
  return parseTable(r.stdout.toString());
}

function parseTable(text: string): [number, number][] {
  const out: [number, number][] = [];
  for (const line of text.split("\n")) {
    const m = /^\s*(\d+)\s+(\d+)\s*$/.exec(line.replace(/\r$/, ""));
    if (m) out.push([Number(m[1]), Number(m[2])]);
  }
  return out;
}

let kernel32: ReturnType<typeof openKernel32> | null = null;

function openKernel32() {
  return dlopen("kernel32.dll", {
    CreateToolhelp32Snapshot: { args: [FFIType.u32, FFIType.u32], returns: FFIType.ptr },
    Process32FirstW: { args: [FFIType.ptr, FFIType.ptr], returns: FFIType.i32 },
    Process32NextW: { args: [FFIType.ptr, FFIType.ptr], returns: FFIType.i32 },
    CloseHandle: { args: [FFIType.ptr], returns: FFIType.i32 },
  }).symbols;
}

/** The process table through `CreateToolhelp32Snapshot`: a `PROCESSENTRY32W` is 568 bytes, with the
 * process id at offset 8 and its creator's at offset 32. */
function windowsTable(): [number, number][] {
  kernel32 ??= openKernel32();
  const snap = kernel32.CreateToolhelp32Snapshot(0x2, 0);
  if (!snap || snap === -1 || Number(snap) >= 2 ** 63) throw new Error("CreateToolhelp32Snapshot failed");
  const entry = new Uint8Array(568);
  const view = new DataView(entry.buffer);
  const out: [number, number][] = [];
  try {
    view.setUint32(0, entry.length, true);
    for (let ok = kernel32.Process32FirstW(snap, ptr(entry)); ok; ok = kernel32.Process32NextW(snap, ptr(entry))) {
      out.push([view.getUint32(8, true), view.getUint32(32, true)]);
      view.setUint32(0, entry.length, true);
    }
  } finally {
    kernel32.CloseHandle(snap);
  }
  return out;
}

/** Every process whose chain of creators leads to `pid`, `pid` itself excluded, nearest first. */
export function descendants(pid: number, table: [number, number][] = processTable()): number[] {
  const children = new Map<number, number[]>();
  for (const [p, parent] of table) if (p !== parent) (children.get(parent) ?? children.set(parent, []).get(parent)!).push(p);
  const out: number[] = [];
  const seen = new Set([pid]);
  for (let queue = [pid]; queue.length > 0; ) {
    const next: number[] = [];
    for (const p of queue) {
      for (const c of children.get(p) ?? []) {
        if (seen.has(c)) continue;
        seen.add(c);
        out.push(c);
        next.push(c);
      }
    }
    queue = next;
  }
  return out;
}

function kill(pid: number): void {
  try {
    process.kill(pid, "SIGKILL");
  } catch {
    // Already gone.
  }
}

/** Kills `pid` and every process it started. The tree is read before the first kill, so a process
 * started in between is the only one it can miss. */
export function killTree(pid: number | undefined): void {
  if (pid === undefined) return;
  let below: number[] = [];
  try {
    below = descendants(pid);
  } catch {
    // Without a table, the one process is still killed.
  }
  kill(pid);
  for (const p of below) kill(p);
}

/** Kills what an exited `pid` left running; returns how many. */
export function reapOrphans(pid: number | undefined): number {
  if (pid === undefined) return 0;
  let below: number[] = [];
  try {
    below = descendants(pid);
  } catch {
    return 0;
  }
  for (const p of below) kill(p);
  return below.length;
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
  const env = childEnv(opts.env);
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
    killTree(child.pid);
  }, opts.timeoutMs ?? DEFAULT_TIMEOUT_MS);
  try {
    const code = await child.exited;
    return timedOut ? 124 : code;
  } finally {
    clearTimeout(timer);
  }
}
