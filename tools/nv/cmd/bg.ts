// `bun nv bg <command…>`: starts a long command detached and prints its job id, so a session goes on
// working while a build or a test run takes its minutes. `bun nv bg --wait <id>` blocks until the job
// ends, prints the last lines of its output and exits with the job's own exit status. `bun nv bg
// --list` names the jobs still running. A `--` ahead of the command is dropped, so a command that
// opens on a flag is still a command.
//
// A job is a directory `.agent-tmp/bg/<id>/`, which is git-ignored:
//   - `job.json` — the command, the directory it runs in, when it started and its supervisor's pid;
//   - `log` — the command's standard output and standard error, in the order it wrote them;
//   - `exit` — its exit status, written once when it ends, and the one sign a job is over.
// The supervisor is this command again, as `bg --run <id>`, started detached from the caller. It runs
// the command with no shell between, waits on it and writes `exit`, so the job outlives the call that
// started it. A supervisor that is killed first leaves no `exit`, and `--wait` reports that job as
// ended with no status rather than waiting forever. Starting a job deletes each finished job older
// than a week.
//
// Exits 0 when a job starts or the list prints, the job's status for `--wait`, and 2 on a bad argument
// or an id that names no job.

import { dlopen, FFIType, type Pointer } from "bun:ffi";
import { spawn } from "node:child_process";
import { closeSync, existsSync, mkdirSync, openSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

export const summary = "run a long command detached: nv bg <command…> | --wait <id> | --list";

const USAGE = "bun nv bg <command…> | --wait <id> | --list";
export const JOBS = join(ROOT, ".agent-tmp", "bg");
/** How many of a job's last output lines `--wait` prints. */
const TAIL = 40;
/** How often `--wait` looks for the job's `exit`. */
const POLL_MS = 200;
const KEEP_MS = 7 * 24 * 60 * 60 * 1000;

interface Job {
  argv: string[];
  cwd: string;
  started: string;
  pid?: number;
}

export const dirOf = (id: string) => join(JOBS, id);

export function readJob(id: string): Job | null {
  const path = join(dirOf(id), "job.json");
  return existsSync(path) ? (JSON.parse(readFileSync(path, "utf8")) as Job) : null;
}

export function exitOf(id: string): number | null {
  const path = join(dirOf(id), "exit");
  return existsSync(path) ? Number(readFileSync(path, "utf8").trim()) : null;
}

export function alive(pid: number | undefined): boolean {
  if (pid === undefined) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function newId(): string {
  const t = new Date().toISOString().replace(/[-:]/g, "").replace("T", "-").slice(0, 15);
  return `${t}-${Math.random().toString(16).slice(2, 6)}`;
}

function prune(): void {
  if (!existsSync(JOBS)) return;
  const cutoff = Date.now() - KEEP_MS;
  for (const id of readdirSync(JOBS)) {
    const exit = join(dirOf(id), "exit");
    if (existsSync(exit) && statSync(exit).mtimeMs < cutoff) rmSync(dirOf(id), { recursive: true, force: true });
  }
}

function start(argv: string[]): number {
  console.log(startJob(argv));
  return 0;
}

/** The handle values `uninheritAll` tries: every multiple of 4 below this. */
const HANDLE_SCAN_END = 1 << 18;

/**
 * Clears the inherit flag on every handle this process has, on Windows, so the supervisor started next
 * inherits none of them. A Windows child inherits every inheritable handle of its parent, whatever its
 * `stdio` says. `bun run`'s shell leaves inheritable copies of the pipes it reads the script's output
 * from in this process, beside the standard handles, so under `bun nv … | tail` the supervisor held
 * that pipe open and the caller waited for the whole job. The supervisor writes only to its job's log.
 * Handle values are multiples of 4, and the flag call fails harmlessly on a value that names no handle.
 * This process only starts the supervisor and then prints and exits, so its own later children lose
 * nothing.
 */
function uninheritAll(): void {
  if (process.platform !== "win32") return;
  try {
    const k32 = dlopen("kernel32.dll", {
      SetHandleInformation: { args: [FFIType.ptr, FFIType.u32, FFIType.u32], returns: FFIType.i32 },
    });
    const HANDLE_FLAG_INHERIT = 1;
    for (let h = 4; h < HANDLE_SCAN_END; h += 4) k32.symbols.SetHandleInformation(h as unknown as Pointer, HANDLE_FLAG_INHERIT, 0);
    k32.close();
  } catch {
    // The job still starts; a caller reading this process through a pipe then waits until it ends.
  }
}

/** Starts `argv` as a detached job and returns its id. */
export function startJob(argv: string[]): string {
  prune();
  const id = newId();
  const dir = dirOf(id);
  mkdirSync(dir, { recursive: true });
  const job: Job = { argv, cwd: process.cwd(), started: new Date().toISOString() };
  writeFileSync(join(dir, "job.json"), JSON.stringify(job, null, 2) + "\n");
  uninheritAll();
  const child = spawn(process.execPath, [join(ROOT, "tools", "nv", "main.ts"), "bg", "--run", id], {
    cwd: ROOT,
    detached: true,
    stdio: "ignore",
    windowsHide: true,
  });
  child.unref();
  // Written whole and then renamed, because the supervisor may already be reading this file.
  const tmp = join(dir, "job.json.tmp");
  writeFileSync(tmp, JSON.stringify({ ...job, pid: child.pid }, null, 2) + "\n");
  renameSync(tmp, join(dir, "job.json"));
  return id;
}

/** The supervisor: runs the job's command into its log and writes its exit status. */
async function supervise(id: string): Promise<number> {
  const job = readJob(id);
  if (job === null) return 2;
  const log = openSync(join(dirOf(id), "log"), "a");
  let code: number;
  try {
    const child = Bun.spawn(job.argv, { cwd: job.cwd, stdin: "ignore", stdout: log, stderr: log, windowsHide: true });
    code = await child.exited;
  } catch (e) {
    writeFileSync(join(dirOf(id), "log"), `nv bg: ${job.argv[0]} did not start: ${(e as Error).message}\n`, { flag: "a" });
    code = 127;
  } finally {
    closeSync(log);
  }
  // Written whole and then renamed, so a waiter never reads a half-written status.
  const tmp = join(dirOf(id), "exit.tmp");
  writeFileSync(tmp, `${code}\n`);
  renameSync(tmp, join(dirOf(id), "exit"));
  return 0;
}

function tail(id: string): string[] {
  const path = join(dirOf(id), "log");
  if (!existsSync(path)) return [];
  const lines = readFileSync(path, "utf8").replace(/\r\n/g, "\n").replace(/\n$/, "").split("\n");
  return lines.length === 1 && lines[0] === "" ? [] : lines.slice(-TAIL);
}

async function wait(id: string): Promise<number> {
  const job = readJob(id);
  if (job === null) {
    console.error(`nv bg: no job \`${id}\` under .agent-tmp/bg/`);
    return 2;
  }
  let code = exitOf(id);
  while (code === null) {
    if (!alive(job.pid)) {
      // The supervisor may have written `exit` between the two looks.
      code = exitOf(id);
      if (code !== null) break;
      for (const l of tail(id)) console.log(l);
      console.log(`bg ${id}: ended with no exit status; its supervisor was stopped first`);
      return 1;
    }
    await Bun.sleep(POLL_MS);
    code = exitOf(id);
  }
  for (const l of tail(id)) console.log(l);
  const secs = Math.round((statSync(join(dirOf(id), "exit")).mtimeMs - Date.parse(job.started)) / 1000);
  console.log(`bg ${id}: exit ${code} after ${secs}s: ${job.argv.join(" ")}`);
  return code;
}

function list(): number {
  const running = (existsSync(JOBS) ? readdirSync(JOBS) : [])
    .sort()
    .map((id) => ({ id, job: readJob(id) }))
    .filter(({ id, job }) => job !== null && exitOf(id) === null && alive(job.pid));
  for (const { id, job } of running) console.log(`${id}  ${job!.argv.join(" ")}`);
  console.log(`bg: ${running.length} job(s) running`);
  return 0;
}

export async function run(args: string[]): Promise<number> {
  const [first, second] = args;
  if (first === "--list" && args.length === 1) return list();
  if (first === "--wait" && args.length === 2) return wait(second!);
  if (first === "--run" && args.length === 2) return supervise(second!);
  const argv = first === "--" ? args.slice(1) : args;
  if (argv.length === 0 || (first !== "--" && first!.startsWith("--"))) {
    console.error(`usage: ${USAGE}`);
    return 2;
  }
  return start(argv);
}
