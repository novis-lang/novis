// The Windows calls: a Job Object that holds a process tree, and what the kit reads of a process by its
// pid. They go to `kernel32` and `ntdll` through `bun:ffi`, and assume a 64-bit process.
//
// - `spawnTree` starts a command suspended, puts it in a fresh job with the kill on close, and only then
//   lets it run, so no grandchild can start outside the job. The job allows no breakaway. Closing the
//   tree, or this process dying, kills every process in it.
// - A tree's job may have a name. `killJob` then ends it from any process, and `jobExists` tells
//   whether a job holds the name.
// - The child inherits exactly the handles it is given: as stdin `NUL`, or a pipe with `input`, and as
//   stdout and stderr the log file, a pipe, or `NUL`. A tree started with `split` gets a second pipe as
//   its stderr. A handle list on the spawn keeps every other inheritable handle of this process out of it.
// - `Tree.read` takes what the pipe holds now and never blocks, because a blocking read through FFI
//   would stall Bun's event loop. It peeks first, and reads only the bytes the peek saw.
// - `Tree.write` never blocks either. The stdin pipe's end is in `PIPE_NOWAIT` mode, so a write takes
//   what the pipe's buffer has room for, and says how much that was. `Tree.closeInput` ends the child's
//   input.
// - A spawn quotes each word of the argv as the C runtime reads it back, unless `line` gives the
//   whole command line, as a `.cmd` script run through `cmd /c` needs.
// - `Tree.freeze` and `Tree.thaw` reach every process in the job through `NtSuspendProcess` and
//   `NtResumeProcess`. A query of the job lists at most `MAX_PROCS` processes, so a freeze or a thaw
//   reaches only those, and says how many it reached. `Tree.kill` terminates the job. `Tree.release`
//   clears the kill on close, so the tree outlives its handles.
// - `spawnDetached` starts a command that inherits no handle at all, out of this process's job when that
//   job allows breakaway, and says when it could not. It holds none of its caller's pipes open.
// - `ctrlBreak` sends Ctrl+Break to a process group that shares this process's console.
// - `alive` and `started` read one process, and `terminate` ends one process alone. A start time
//   tells a process apart from a later one that the OS gave the same pid.
// - `killTree` ends a process Bun started and every process below it, through `taskkill /T`, for a
//   child that needs a pipe and so cannot be a `Tree`.
// - A tree started with `hide` gets `SW_SHOWNOACTIVATE` as its start options while it is suspended, so
//   its first window opens without the focus. The options sit in the process's own memory, so only
//   that process gets them. `Tree.watchWindows` keeps every window of the tree off the screen
//   (`windows-desktop.ts`).
// - `Mutex` is a named mutex of the logon session. The OS releases it when its holder dies, so a crash
//   never leaves it taken. It is taken without waiting, since a wait through FFI would stall Bun.
// - `listeners` reads every listening TCP port with its pid, and `imageName` a process's program.
// - `freeBytes` reads the bytes free to this user on a path's volume, for the disk gate of
//   `[loop] min_free_gb`.
// - `noErrorBoxes` turns off Windows' critical-error dialogs for this process and what it starts.
//
// Nothing here is checked by the compiler against the Windows headers. Each struct offset is written
// for x64, and the tests in `os.test.ts` are the only guard.

import { basename, join } from "node:path";
import { dlopen, FFIType, type Pointer, ptr } from "bun:ffi";
import { showsWindow, type WindowWatcher, watchWindows } from "./windows-desktop.ts";

/** The most processes a query of a job lists. */
export const MAX_PROCS = 256;

const CREATE_SUSPENDED = 0x4;
const CREATE_NEW_PROCESS_GROUP = 0x200;
const CREATE_UNICODE_ENVIRONMENT = 0x400;
const EXTENDED_STARTUPINFO_PRESENT = 0x80000;
const CREATE_BREAKAWAY_FROM_JOB = 0x1000000;
const CREATE_NO_WINDOW = 0x8000000;
const STARTF_USESTDHANDLES = 0x100;
const PROC_THREAD_ATTRIBUTE_HANDLE_LIST = 0x20002n;
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x2000;
const JOB_BASIC_PROCESS_ID_LIST = 3;
const JOB_EXTENDED_LIMIT_INFORMATION = 9;
const PROCESS_TERMINATE = 0x1;
const PROCESS_SUSPEND_RESUME = 0x800;
const PROCESS_QUERY_LIMITED_INFORMATION = 0x1000;
const SYNCHRONIZE = 0x100000;
const STILL_ACTIVE = 259;
const CTRL_BREAK_EVENT = 1;
const GENERIC_READ = 0x80000000;
const GENERIC_WRITE = 0x40000000;
const FILE_APPEND_DATA = 0x4;
const SHARE_ALL = 0x7;
const HANDLE_FLAG_INHERIT = 0x1;
const JOB_OBJECT_QUERY = 0x4;
const JOB_OBJECT_TERMINATE = 0x8;
/** The size the pipe of a tree asks the OS for. */
const PIPE_BYTES = 1 << 20;
const PIPE_NOWAIT = 0x1;
/** The most bytes one `Tree.read` takes. */
export const READ_BYTES = 1 << 20;
const OPEN_EXISTING = 3;
const OPEN_ALWAYS = 4;
const WAIT_ABANDONED = 0x80;
const SEM_FAILCRITICALERRORS = 0x1;
const STARTF_USESHOWWINDOW = 0x1;
const SW_SHOWNOACTIVATE = 4;
/** The offsets, in a 64-bit process, of `ProcessParameters` in the PEB and of the window flags in it. */
const PEB_PARAMS = 0x20n;
const WINDOW_FLAGS = 0xa4n;
const SHOW_WINDOW_FLAGS = 0xa8n;
const AF_INET = 2;
const AF_INET6 = 23;
const TCP_TABLE_OWNER_PID_LISTENER = 3;
/** A FILETIME counts 100 ns ticks from 1601. The Unix epoch is this many ticks later. */
const EPOCH_TICKS = 116_444_736_000_000_000n;

const { ptr: P, i32, u32, u64, i64, void: VOID } = FFIType;

let libs: ReturnType<typeof open> | undefined;

function open() {
  const k = dlopen("kernel32.dll", {
    CreateJobObjectW: { args: [P, P], returns: P },
    OpenJobObjectW: { args: [u32, i32, P], returns: P },
    CreatePipe: { args: [P, P, P, u32], returns: i32 },
    SetHandleInformation: { args: [P, u32, u32], returns: i32 },
    PeekNamedPipe: { args: [P, P, u32, P, P, P], returns: i32 },
    ReadFile: { args: [P, P, u32, P, P], returns: i32 },
    WriteFile: { args: [P, P, u32, P, P], returns: i32 },
    SetNamedPipeHandleState: { args: [P, P, P, P], returns: i32 },
    SetInformationJobObject: { args: [P, i32, P, u32], returns: i32 },
    QueryInformationJobObject: { args: [P, i32, P, u32, P], returns: i32 },
    AssignProcessToJobObject: { args: [P, P], returns: i32 },
    TerminateJobObject: { args: [P, u32], returns: i32 },
    TerminateProcess: { args: [P, u32], returns: i32 },
    CloseHandle: { args: [P], returns: i32 },
    OpenProcess: { args: [u32, i32, u32], returns: P },
    GetExitCodeProcess: { args: [P, P], returns: i32 },
    GetProcessTimes: { args: [P, P, P, P, P], returns: i32 },
    CreateProcessW: { args: [P, P, P, P, i32, u32, P, P, P, P], returns: i32 },
    CreateFileW: { args: [P, u32, u32, P, u32, u32, P], returns: i64 },
    InitializeProcThreadAttributeList: { args: [P, u32, u32, P], returns: i32 },
    UpdateProcThreadAttribute: { args: [P, u32, u64, P, u64, P, P], returns: i32 },
    DeleteProcThreadAttributeList: { args: [P], returns: VOID },
    GenerateConsoleCtrlEvent: { args: [u32, u32], returns: i32 },
    ReadProcessMemory: { args: [P, u64, P, u64, P], returns: i32 },
    WriteProcessMemory: { args: [P, u64, P, u64, P], returns: i32 },
    CreateMutexW: { args: [P, i32, P], returns: P },
    WaitForSingleObject: { args: [P, u32], returns: u32 },
    ReleaseMutex: { args: [P], returns: i32 },
    QueryFullProcessImageNameW: { args: [P, u32, P, P], returns: i32 },
    GetErrorMode: { args: [], returns: u32 },
    GetDiskFreeSpaceExW: { args: [P, P, P, P], returns: i32 },
    SetErrorMode: { args: [u32], returns: u32 },
  });
  const nt = dlopen("ntdll.dll", {
    NtSuspendProcess: { args: [P], returns: i32 },
    NtResumeProcess: { args: [P], returns: i32 },
    NtQueryInformationProcess: { args: [P, u32, P, u32, P], returns: i32 },
  });
  return { k: k.symbols, nt: nt.symbols };
}

let iphlp: ReturnType<typeof openIphlp> | undefined;

function openIphlp() {
  return dlopen("iphlpapi.dll", {
    GetExtendedTcpTable: { args: [P, P, i32, u32, u32, u32], returns: u32 },
  }).symbols;
}

/** The libraries, opened on first use so that importing this file on another OS does not fail. */
function lib() {
  libs ??= open();
  return libs;
}

/** `s` as a NUL-terminated UTF-16 string. */
function wide(s: string): Buffer {
  return Buffer.from(`${s}\0`, "utf16le");
}

/** The pointer stored at `offset` of `buf`. */
function handleAt(buf: Buffer, offset: number): Pointer {
  return Number(buf.readBigUInt64LE(offset)) as Pointer;
}

function close(h: Pointer | null): void {
  if (h) lib().k.CloseHandle(h);
}

/** `arg` as one word of a Windows command line, quoted as the C runtime reads it back. */
export function quote(arg: string): string {
  if (arg !== "" && !/[ \t"]/.test(arg)) return arg;
  let out = '"';
  let slashes = 0;
  for (const c of arg) {
    if (c === "\\") {
      slashes++;
      continue;
    }
    out += "\\".repeat(c === '"' ? slashes * 2 + 1 : slashes) + c;
    slashes = 0;
  }
  return `${out}${"\\".repeat(slashes * 2)}"`;
}

/** A Unicode environment block: each `key=value` NUL-terminated, then one more NUL. */
function envBlock(env: Record<string, string | undefined>): Buffer {
  const lines = Object.entries(env)
    .filter(([, v]) => v !== undefined)
    .map(([k, v]) => `${k}=${v}\0`);
  return Buffer.from(`${lines.join("")}\0\0`, "utf16le");
}

/** How to start a command. */
export interface SpawnOptions {
  /** The working directory. This process's when absent. */
  cwd?: string;
  /** Exactly the variables the command sees. This process's when absent. */
  env?: Record<string, string | undefined>;
  /** The whole command line, in place of the argv's words quoted. */
  line?: string;
}

/** How to start a tree. */
export interface TreeOptions extends SpawnOptions {
  /** A file the tree's stdout and stderr append to. `NUL` when absent, and ignored with `pipe`. */
  log?: string;
  /** Sends the tree's stdout and stderr to a pipe that `Tree.read` drains. */
  pipe?: boolean;
  /** With `pipe`, sends the tree's stderr to a second pipe, which `Tree.readErr` drains. */
  split?: boolean;
  /** Gives the tree a pipe as its stdin, which `Tree.write` fills. */
  input?: boolean;
  /** The name of the tree's job, which `killJob` reaches. A name another job holds is refused. */
  name?: string;
  /** Starts the tree as a process group of its own, which `ctrlBreak` reaches by its pid. */
  group?: boolean;
  /** Opens the first process's first window without the focus. */
  hide?: boolean;
}

interface Created {
  process: Pointer;
  pid: number;
}

/** CreateProcessW, closing the thread handle. Gives null when the call fails. */
function create(
  cmd: string[],
  opts: SpawnOptions,
  flags: number,
  info: Buffer,
  inherit: boolean,
): Created | null {
  const line = wide(opts.line ?? cmd.map(quote).join(" "));
  const env = envBlock(opts.env ?? process.env);
  const cwd = wide(opts.cwd ?? process.cwd());
  const out = Buffer.alloc(24);
  const ok = lib().k.CreateProcessW(
    null,
    ptr(line),
    null,
    null,
    inherit ? 1 : 0,
    flags | CREATE_UNICODE_ENVIRONMENT,
    ptr(env),
    ptr(cwd),
    ptr(info),
    ptr(out),
  );
  if (ok === 0) return null;
  close(handleAt(out, 8));
  return { process: handleAt(out, 0), pid: out.readUInt32LE(16) };
}

/** Opens `path` as an inheritable handle: for appending when `append`, else for reading and writing. */
function inheritable(path: string, append: boolean): Pointer {
  const sa = Buffer.alloc(24);
  sa.writeUInt32LE(24, 0);
  sa.writeInt32LE(1, 16);
  const access = append ? FILE_APPEND_DATA | SYNCHRONIZE : GENERIC_READ | GENERIC_WRITE;
  const h = lib().k.CreateFileW(ptr(wide(path)), access, SHARE_ALL, ptr(sa), append ? OPEN_ALWAYS : OPEN_EXISTING, 0, null);
  if (h === -1n) throw new Error(`cannot open ${path} for the tree`);
  return Number(h) as Pointer;
}

function exitOf(h: Pointer): number | null {
  const code = Buffer.alloc(4);
  if (lib().k.GetExitCodeProcess(h, ptr(code)) === 0) return null;
  const c = code.readUInt32LE(0);
  return c === STILL_ACTIVE ? null : c;
}

/**
 * A process tree in a Job Object. It holds its first process by a handle, so a pid the OS gives out
 * again never stands for it.
 */
export class Tree {
  /** When the tree's first process started, in ms since the Unix epoch. */
  readonly started: number;
  private job: Pointer | null;

  constructor(
    job: Pointer,
    private process: Pointer,
    readonly pid: number,
    private pipe: Pointer | null = null,
    private errPipe: Pointer | null = null,
    private inPipe: Pointer | null = null,
  ) {
    this.job = job;
    this.started = startOf(process) ?? 0;
  }

  /**
   * What the tree's pipe holds now, at most `READ_BYTES`, without waiting: empty while it holds
   * nothing, and null once every process that could write to it has closed it. Null for a tree with
   * no pipe.
   */
  read(): Buffer | null {
    return readPipe(this.pipe);
  }

  /** What the tree's stderr pipe holds now, as `read` gives it. Null for a tree started without `split`. */
  readErr(): Buffer | null {
    return readPipe(this.errPipe);
  }

  /**
   * Writes what of `bytes` the stdin pipe has room for now, without waiting, and says how many bytes
   * that was. Null once the input is closed or the child has closed its end.
   */
  write(bytes: Uint8Array): number | null {
    if (!this.inPipe) return null;
    if (bytes.length === 0) return 0;
    const buf = Buffer.from(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const put = Buffer.alloc(4);
    if (lib().k.WriteFile(this.inPipe, ptr(buf), buf.length, ptr(put), null) === 0) {
      this.closeInput();
      return null;
    }
    return put.readUInt32LE(0);
  }

  /** Closes the tree's stdin pipe, so the child reads the end of its input. */
  closeInput(): void {
    close(this.inPipe);
    this.inPipe = null;
  }

  /** The first process's exit code once it has exited, or null while it runs. */
  exitCode(): number | null {
    return exitOf(this.process);
  }

  /** Resolves with the first process's exit code once it exits, polling every `everyMs`. */
  async exited(everyMs = 25): Promise<number> {
    for (;;) {
      const code = this.exitCode();
      if (code !== null) return code;
      await Bun.sleep(everyMs);
    }
  }

  /** The pids of the processes in the job now, at most `MAX_PROCS` of them. */
  pids(): number[] {
    return this.job ? pidsOf(this.job) : [];
  }

  /** Suspends every process in the job. Returns how many it suspended. */
  freeze(): number {
    return this.each((h) => lib().nt.NtSuspendProcess(h));
  }

  /** Resumes every process in the job. Returns how many it resumed. */
  thaw(): number {
    return this.each((h) => lib().nt.NtResumeProcess(h));
  }

  private each(call: (h: Pointer) => number): number {
    let reached = 0;
    for (const pid of this.pids()) {
      const h = lib().k.OpenProcess(PROCESS_SUSPEND_RESUME, 0, pid) as Pointer | null;
      if (!h) continue;
      if (call(h) >= 0) reached++;
      close(h);
    }
    return reached;
  }

  /** Terminates every process in the job, each with exit code `code`. */
  kill(code = 1): void {
    if (this.job && lib().k.TerminateJobObject(this.job, code) === 0) throw new Error("cannot terminate the job");
  }

  /** Starts keeping every window of the tree off the screen, until the watcher is stopped. */
  watchWindows(): WindowWatcher {
    return watchWindows(() => this.pids());
  }

  /** Whether a process of the tree shows a visible top-level window, on the screen or off it. */
  showsWindow(): boolean {
    return showsWindow(this.pids());
  }

  /** Clears the kill on close, so the tree's processes outlive the job's handles. */
  release(): void {
    if (this.job) limit(this.job, 0);
  }

  /** Closes the tree's handles. Unless it was released, that kills every process left in it. */
  close(): void {
    close(this.job);
    close(this.process);
    close(this.pipe);
    close(this.errPipe);
    this.closeInput();
    this.job = null;
    this.pipe = null;
    this.errPipe = null;
  }
}

/** What the pipe `h` holds now, as `Tree.read` gives it. */
function readPipe(h: Pointer | null): Buffer | null {
  if (!h) return null;
  const { k } = lib();
  const avail = Buffer.alloc(4);
  if (k.PeekNamedPipe(h, null, 0, null, ptr(avail), null) === 0) return null;
  const n = Math.min(avail.readUInt32LE(0), READ_BYTES);
  if (n === 0) return Buffer.alloc(0);
  const buf = Buffer.alloc(n);
  const got = Buffer.alloc(4);
  if (k.ReadFile(h, ptr(buf), n, ptr(got), null) === 0) return null;
  return buf.subarray(0, got.readUInt32LE(0));
}

/**
 * A pipe of which the child inherits one end: the write end of its output, or the read end of its
 * input. The end this process keeps of an input pipe never blocks a write.
 */
function pipe(input = false): { read: Pointer; write: Pointer } {
  const { k } = lib();
  const ends = Buffer.alloc(16);
  if (k.CreatePipe(ptr(ends), ptr(ends, 8), null, PIPE_BYTES) === 0) throw new Error("cannot make the tree's pipe");
  const read = handleAt(ends, 0);
  const write = handleAt(ends, 8);
  const mode = Buffer.alloc(4);
  mode.writeUInt32LE(PIPE_NOWAIT, 0);
  const set =
    k.SetHandleInformation(input ? read : write, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) !== 0 &&
    (!input || k.SetNamedPipeHandleState(write, ptr(mode), null, null) !== 0);
  if (!set) {
    close(read);
    close(write);
    throw new Error("cannot make the pipe inheritable");
  }
  return { read, write };
}

/** Opens the job named `name` with `access`, or gives null when no job holds the name. */
function openJob(name: string, access: number): Pointer | null {
  return lib().k.OpenJobObjectW(access, 0, ptr(wide(name))) as Pointer | null;
}

/** The pids of the processes in `job` now, at most `MAX_PROCS` of them. */
function pidsOf(job: Pointer): number[] {
  const list = Buffer.alloc(8 + 8 * MAX_PROCS);
  // A job with more processes than fit fails with ERROR_MORE_DATA and still fills what fits.
  lib().k.QueryInformationJobObject(job, JOB_BASIC_PROCESS_ID_LIST, ptr(list), list.length, null);
  const n = Math.min(list.readUInt32LE(4), MAX_PROCS);
  return Array.from({ length: n }, (_, i) => Number(list.readBigUInt64LE(8 + 8 * i)));
}

/** The pids of the processes in the job named `name`, empty when no job holds the name. */
export function jobPids(name: string): number[] {
  const h = openJob(name, JOB_OBJECT_QUERY);
  if (!h) return [];
  const pids = pidsOf(h);
  close(h);
  return pids;
}

/** Whether a job holds the name `name`. */
export function jobExists(name: string): boolean {
  const h = openJob(name, JOB_OBJECT_QUERY);
  close(h);
  return h !== null;
}

/** Terminates every process of the job named `name`, each with exit code `code`. Says whether it did. */
export function killJob(name: string, code = 1): boolean {
  const h = openJob(name, JOB_OBJECT_TERMINATE);
  if (!h) return false;
  const ok = lib().k.TerminateJobObject(h, code) !== 0;
  close(h);
  return ok;
}

function limit(job: Pointer, flags: number): void {
  const limits = Buffer.alloc(144);
  limits.writeUInt32LE(flags, 16);
  if (lib().k.SetInformationJobObject(job, JOB_EXTENDED_LIMIT_INFORMATION, ptr(limits), limits.length) === 0) {
    throw new Error("cannot set the job's limits");
  }
}

/** Starts `cmd` as a tree in a fresh Job Object that kills it on close. */
export function spawnTree(cmd: string[], opts: TreeOptions = {}): Tree {
  const { k, nt } = lib();
  if (opts.name && jobExists(opts.name)) throw new Error(`a job already holds the name ${opts.name}`);
  const job = k.CreateJobObjectW(null, opts.name ? ptr(wide(opts.name)) : null) as Pointer | null;
  if (!job) throw new Error("cannot create a job");
  let ends: { read: Pointer; write: Pointer } | null = null;
  let errEnds: { read: Pointer; write: Pointer } | null = null;
  let inEnds: { read: Pointer; write: Pointer } | null = null;
  let nul: Pointer;
  try {
    ends = opts.pipe ? pipe() : null;
    errEnds = opts.pipe && opts.split ? pipe() : null;
    inEnds = opts.input ? pipe(true) : null;
    nul = inheritable("NUL", false);
  } catch (e) {
    for (const end of [ends, errEnds, inEnds]) {
      if (end) close(end.read);
      if (end) close(end.write);
    }
    close(job);
    throw e;
  }
  const log = ends ? ends.write : opts.log ? inheritable(opts.log, true) : nul;
  const errLog = errEnds ? errEnds.write : log;
  const input = inEnds ? inEnds.read : nul;
  const handles = [input, nul, log, errLog].filter((h, i, all) => all.indexOf(h) === i);
  const list = Buffer.alloc(8 * handles.length);
  handles.forEach((h, i) => list.writeBigUInt64LE(BigInt(h), 8 * i));
  const size = Buffer.alloc(8);
  k.InitializeProcThreadAttributeList(null, 1, 0, ptr(size));
  const attrs = Buffer.alloc(Number(size.readBigUInt64LE(0)));
  const info = Buffer.alloc(112);
  let made: Created | null = null;
  try {
    limit(job, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE);
    if (k.InitializeProcThreadAttributeList(ptr(attrs), 1, 0, ptr(size)) === 0) throw new Error("cannot make an attribute list");
    try {
      const set = k.UpdateProcThreadAttribute(ptr(attrs), 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, ptr(list), BigInt(list.length), null, null);
      if (set === 0) throw new Error("cannot set the handle list");
      info.writeUInt32LE(112, 0);
      info.writeUInt32LE(STARTF_USESTDHANDLES, 60);
      info.writeBigUInt64LE(BigInt(input), 80);
      info.writeBigUInt64LE(BigInt(log), 88);
      info.writeBigUInt64LE(BigInt(errLog), 96);
      info.writeBigUInt64LE(BigInt(ptr(attrs)), 104);
      const flags = CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT | (opts.group ? CREATE_NEW_PROCESS_GROUP : 0);
      made = create(cmd, opts, flags, info, true);
    } finally {
      k.DeleteProcThreadAttributeList(ptr(attrs));
    }
    if (!made) throw new Error(`cannot start ${cmd[0]}`);
    // Options that cannot be set leave the watcher alone to hide the windows.
    if (opts.hide) noActivate(made.process);
    if (k.AssignProcessToJobObject(job, made.process) === 0) {
      k.TerminateJobObject(job, 1);
      throw new Error(`cannot put ${cmd[0]} in its job`);
    }
    if (nt.NtResumeProcess(made.process) < 0) throw new Error(`cannot resume ${cmd[0]}`);
    return new Tree(job, made.process, made.pid, ends?.read ?? null, errEnds?.read ?? null, inEnds?.write ?? null);
  } catch (e) {
    if (made) {
      k.TerminateJobObject(job, 1);
      close(made.process);
    }
    close(job);
    if (ends) close(ends.read);
    if (errEnds) close(errEnds.read);
    if (inEnds) close(inEnds.write);
    throw e;
  } finally {
    for (const h of handles) close(h);
  }
}

/** A process started detached, which this process does not hold. */
export interface Detached {
  pid: number;
  /** When it started, in ms since the Unix epoch. */
  started: number;
  /** Whether it stayed in this process's job, which allows no breakaway, and so ends with that job. */
  bound: boolean;
}

/**
 * Starts `cmd` with no handle of this process, not even stdin, stdout and stderr, in a process group of
 * its own and with no window. It leaves this process's job when the job allows it.
 */
export function spawnDetached(cmd: string[], opts: SpawnOptions = {}): Detached {
  const info = Buffer.alloc(104);
  info.writeUInt32LE(104, 0);
  const flags = CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW;
  const away = create(cmd, opts, flags | CREATE_BREAKAWAY_FROM_JOB, info, false);
  const made = away ?? create(cmd, opts, flags, info, false);
  if (!made) throw new Error(`cannot start ${cmd[0]}`);
  const started = startOf(made.process) ?? 0;
  close(made.process);
  return { pid: made.pid, started, bound: away === null };
}

/** Sends Ctrl+Break to the process group `group`, which must share this process's console. */
export function ctrlBreak(group: number): void {
  if (lib().k.GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, group) === 0) {
    throw new Error(`cannot send Ctrl+Break to group ${group}`);
  }
}

/** Opens the running process `pid` with `access`, or gives null when it has exited or cannot be opened. */
function openRunning(pid: number, access: number): Pointer | null {
  const h = lib().k.OpenProcess(access | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) as Pointer | null;
  if (h && exitOf(h) === null) return h;
  close(h);
  return null;
}

/** Whether the process `pid` is running. */
export function alive(pid: number): boolean {
  const h = openRunning(pid, 0);
  close(h);
  return h !== null;
}

/**
 * When the running process `pid` started, in ms since the Unix epoch, or null when it has exited or
 * cannot be read. A pid the OS gave out again names a process that started later.
 */
export function started(pid: number): number | null {
  const h = openRunning(pid, 0);
  if (!h) return null;
  const at = startOf(h);
  close(h);
  return at;
}

function startOf(h: Pointer): number | null {
  const times = Buffer.alloc(32);
  const p = ptr(times);
  if (lib().k.GetProcessTimes(h, p, ptr(times, 8), ptr(times, 16), ptr(times, 24)) === 0) return null;
  return Number((times.readBigUInt64LE(0) - EPOCH_TICKS) / 10_000n);
}

/** Terminates the running process `pid` alone, with exit code `code`. */
export function terminate(pid: number, code = 1): boolean {
  const h = openRunning(pid, PROCESS_TERMINATE);
  if (!h) return false;
  const ok = lib().k.TerminateProcess(h, code) !== 0;
  close(h);
  return ok;
}

/** Sets the start options of the suspended `process`, so its first window shows with `SW_SHOWNOACTIVATE`. Says whether it did. */
function noActivate(process: Pointer): boolean {
  const { k, nt } = lib();
  const at = (addr: bigint, bytes: number): Buffer | null => {
    const buf = Buffer.alloc(bytes);
    return k.ReadProcessMemory(process, addr, ptr(buf), BigInt(bytes), null) === 0 ? null : buf;
  };
  const put = (addr: bigint, value: number): boolean => {
    const buf = Buffer.alloc(4);
    buf.writeUInt32LE(value, 0);
    return k.WriteProcessMemory(process, addr, ptr(buf), 4n, null) !== 0;
  };
  const basic = Buffer.alloc(48);
  if (nt.NtQueryInformationProcess(process, 0, ptr(basic), basic.length, null) < 0) return false;
  const params = at(basic.readBigUInt64LE(8) + PEB_PARAMS, 8)?.readBigUInt64LE(0);
  const flags = params === undefined ? undefined : at(params + WINDOW_FLAGS, 4)?.readUInt32LE(0);
  if (params === undefined || flags === undefined) return false;
  return put(params + WINDOW_FLAGS, flags | STARTF_USESHOWWINDOW) && put(params + SHOW_WINDOW_FLAGS, SW_SHOWNOACTIVATE);
}

/** A named mutex, which a crash of its holder releases. */
export class Mutex {
  private constructor(private h: Pointer | null) {}

  /** Opens the mutex named `name`, making it when no process holds it open. */
  static open(name: string): Mutex {
    const h = lib().k.CreateMutexW(null, 0, ptr(wide(name))) as Pointer | null;
    if (!h) throw new Error(`cannot open the mutex ${name}`);
    return new Mutex(h);
  }

  /** Takes the mutex when it is free, and says whether it did. A holder that died leaves it free. */
  tryTake(): boolean {
    if (!this.h) return false;
    const got = lib().k.WaitForSingleObject(this.h, 0);
    return got === 0 || got === WAIT_ABANDONED;
  }

  /** Releases the mutex, which this thread holds, and closes it. */
  close(): void {
    if (!this.h) return;
    lib().k.ReleaseMutex(this.h);
    close(this.h);
    this.h = null;
  }
}

/** Every TCP port, IPv4 and IPv6, that a process listens on, with its pid. Sorted by port, then pid, without repeats. */
export function listeners(): [number, number][] {
  iphlp ??= openIphlp();
  const found = new Map<string, [number, number]>();
  // An IPv4 row is six u32s and an IPv6 row 56 bytes. Each names its local port and its owner at
  // these offsets, after a u32 count.
  for (const [family, row, port, pid] of [
    [AF_INET, 24, 8, 20],
    [AF_INET6, 56, 20, 52],
  ] as const) {
    const size = Buffer.alloc(4);
    let table: Buffer | null = null;
    for (let tries = 0; tries < 4; tries++) {
      const buf = Buffer.alloc(Math.max(size.readUInt32LE(0), 4));
      if (iphlp.GetExtendedTcpTable(ptr(buf), ptr(size), 0, family, TCP_TABLE_OWNER_PID_LISTENER, 0) === 0) {
        table = buf;
        break;
      }
    }
    if (table === null) continue;
    const count = table.readUInt32LE(0);
    for (let i = 0; i < count && 4 + (i + 1) * row <= table.length; i++) {
      const r = 4 + i * row;
      const entry: [number, number] = [table.readUInt16BE(r + port), table.readUInt32LE(r + pid)];
      found.set(`${entry[0]}:${entry[1]}`, entry);
    }
  }
  return [...found.values()].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
}

/** The file name of the program process `pid` runs, such as `node.exe`, or null when it cannot be read. */
export function imageName(pid: number): string | null {
  const h = lib().k.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) as Pointer | null;
  if (!h) return null;
  const buf = Buffer.alloc(2048);
  const len = Buffer.alloc(4);
  len.writeUInt32LE(1024, 0);
  const ok = lib().k.QueryFullProcessImageNameW(h, 0, ptr(buf), ptr(len)) !== 0;
  close(h);
  return ok ? basename(buf.toString("utf16le", 0, len.readUInt32LE(0) * 2)) : null;
}

/** The bytes free to this user on the volume that holds `path`. Throws when the path names no volume. */
export function freeBytes(path: string): number {
  const free = Buffer.alloc(8);
  if (lib().k.GetDiskFreeSpaceExW(ptr(wide(path)), ptr(free), null, null) === 0) {
    throw new Error(`cannot read the free space of ${path}`);
  }
  return Number(free.readBigUInt64LE(0));
}

/** Shows no dialog for a critical error of this process, or of each process it starts from now on. */
export function noErrorBoxes(): void {
  const { k } = lib();
  k.SetErrorMode(k.GetErrorMode() | SEM_FAILCRITICALERRORS);
}

/** Terminates the process `pid` and every process it started, through `taskkill /T`. Says whether it did. */
export function killTree(pid: number): boolean {
  const root = process.env.SystemRoot ?? "C:\\Windows";
  const ran = Bun.spawnSync([join(root, "System32", "taskkill.exe"), "/T", "/F", "/PID", String(pid)], {
    stdin: "ignore",
    stdout: "ignore",
    stderr: "ignore",
    windowsHide: true,
  });
  return ran.exitCode === 0;
}
