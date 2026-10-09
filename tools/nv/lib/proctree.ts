// A child process and everything it started, as one thing: freeze it, thaw it, kill it, and kill what it
// left behind once it has exited. Every program `nv` starts is wrapped in one the moment `Bun.spawn`
// returns: `proc.run`, `proc.passthrough`, the proof runner, and the loop's session, which `nv loop`
// halts by freezing it, since a frozen agent whose `cargo build` keeps writing `target/` is not halted.
// Carrying on resumes every process at the instruction it stopped on.
//
// Nothing is ever killed by a process id alone. Windows keeps a process's creator id after the creator
// exits and later gives that number to a new process, so a tree read from creator ids alone can reach
// any process on the machine: `wininit.exe` names a boot-time `smss.exe` that exited long ago, and a
// child handed that id has every service, Explorer and the whole desktop below it. So:
//
// - On Windows the child is put in a Job Object in the constructor, which runs before the spawner awaits
//   anything: until then libuv holds the child's handle, so its id is still the child's. Every process it
//   starts from then on is in the job and cannot leave it, so the job's process list is the tree,
//   including a grandchild whose parent has exited, and `TerminateJobObject` reaches nothing outside it.
//   The child's own handle is held until `close`, so Windows cannot give its id to another process
//   before then. A grandchild started in the microseconds between the spawn and the job escapes the job,
//   since `Bun.spawn` cannot start a child suspended. Where no job can be made, only the child is killed,
//   through that handle.
// - Elsewhere the tree is read from creator ids while the child is still running. Linux moves an orphan
//   to init, so a live process's creator id is never stale there; a process still counts only when it
//   started no earlier than the child and no earlier than its creator, and its start is read again just
//   before the kill. The child itself is signalled through the handle its caller owns. It is not started
//   in a process group of its own, because `detached` calls `setsid`, which Ctrl-C in the terminal then
//   no longer reaches. An orphan is therefore out of reach: only a kill taken while the tree is whole
//   reaches it, and `reap` kills nothing.
// - Every process addressed by its id is screened first: never this process, an ancestor of it, or a
//   process that started before it. Each refusal is printed to stderr. The screen is a second check
//   only: which processes belong to the tree comes from the job, or from Linux's own creator links.
// - The child is signalled through its owner's handle only while its exit has not been observed.
//
// The job has no limits, so closing it kills nothing. On Windows each process is frozen with
// `NtSuspendProcess` through a handle that `IsProcessInJob` has confirmed, and that handle is kept and
// resumed through; elsewhere the child alone gets `SIGSTOP` and `SIGCONT`.
//
// What it costs: per spawn one job handle and one process handle, held until `close`, and four calls;
// per frozen process one handle until the thaw; per kill one snapshot of the process table and one
// `OpenProcess` per running process to read when it started.
//
// Every method is best effort and never throws: a freeze that cannot be taken shows in its count, and a
// kill that cannot be made safe kills less, never more.

import { readFileSync } from "node:fs";
import { dlopen, FFIType, ptr, type Pointer } from "bun:ffi";

/** A Windows handle as the FFI hands it back. */
type Handle = Pointer;

const WIN = process.platform === "win32";
const PROCESS_TERMINATE = 0x0001;
const PROCESS_SET_QUOTA = 0x0100;
const PROCESS_SUSPEND_RESUME = 0x0800;
const PROCESS_QUERY_LIMITED_INFORMATION = 0x1000;
const JOB_PROCESS_ID_LIST = 3;
const ERROR_MORE_DATA = 234;
const TH32CS_SNAPPROCESS = 0x2;

type Api = ReturnType<typeof load>;
let api: Api | null | undefined;

function load() {
  const k32 = dlopen("kernel32.dll", {
    CreateJobObjectW: { args: [FFIType.ptr, FFIType.ptr], returns: FFIType.ptr },
    OpenProcess: { args: [FFIType.u32, FFIType.i32, FFIType.u32], returns: FFIType.ptr },
    AssignProcessToJobObject: { args: [FFIType.ptr, FFIType.ptr], returns: FFIType.i32 },
    IsProcessInJob: { args: [FFIType.ptr, FFIType.ptr, FFIType.ptr], returns: FFIType.i32 },
    QueryInformationJobObject: { args: [FFIType.ptr, FFIType.i32, FFIType.ptr, FFIType.u32, FFIType.ptr], returns: FFIType.i32 },
    TerminateJobObject: { args: [FFIType.ptr, FFIType.u32], returns: FFIType.i32 },
    TerminateProcess: { args: [FFIType.ptr, FFIType.u32], returns: FFIType.i32 },
    GetProcessTimes: { args: [FFIType.ptr, FFIType.ptr, FFIType.ptr, FFIType.ptr, FFIType.ptr], returns: FFIType.i32 },
    CreateToolhelp32Snapshot: { args: [FFIType.u32, FFIType.u32], returns: FFIType.ptr },
    Process32FirstW: { args: [FFIType.ptr, FFIType.ptr], returns: FFIType.i32 },
    Process32NextW: { args: [FFIType.ptr, FFIType.ptr], returns: FFIType.i32 },
    CloseHandle: { args: [FFIType.ptr], returns: FFIType.i32 },
    GetLastError: { args: [], returns: FFIType.u32 },
  });
  const nt = dlopen("ntdll.dll", {
    NtSuspendProcess: { args: [FFIType.ptr], returns: FFIType.i32 },
    NtResumeProcess: { args: [FFIType.ptr], returns: FFIType.i32 },
  });
  return { k32: k32.symbols, nt: nt.symbols };
}

/** The Windows calls, or null where they cannot be loaded. */
function win(): Api | null {
  if (api === undefined) {
    try {
      api = WIN ? load() : null;
    } catch {
      api = null;
    }
  }
  return api;
}

// ---- the process table -----------------------------------------------------------------------------

/** One running process: its id, its creator's id, and when it started, or null when that cannot be read. */
export interface Proc {
  pid: number;
  parent: number;
  started: bigint | null;
}

/** When the process behind `handle` started, as a `FILETIME`. */
function handleStarted(handle: Handle): bigint | null {
  const t = new BigUint64Array(4);
  return win()!.k32.GetProcessTimes(handle, ptr(t), ptr(t, 8), ptr(t, 16), ptr(t, 24)) ? t[0]! : null;
}

/**
 * When `pid` started, or null when that cannot be read. The value is only ever compared with another
 * from this function: a `FILETIME` on Windows, clock ticks since boot from `/proc` on Linux, and null
 * everywhere else.
 */
export function startedAt(pid: number): bigint | null {
  const w = win();
  if (w !== null) {
    const handle = w.k32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) as Handle | null;
    if (!handle) return null;
    try {
      return handleStarted(handle);
    } finally {
      w.k32.CloseHandle(handle);
    }
  }
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, "utf8");
    // The fields after the parenthesised name start at field 3; the start time is field 22.
    const field = stat.slice(stat.lastIndexOf(")") + 2).split(" ")[19];
    return field === undefined ? null : BigInt(field);
  } catch {
    return null;
  }
}

/** Each process's id and its creator's id, from one snapshot of the process table, or none. */
function ids(): [number, number][] {
  const w = win();
  if (w !== null) {
    // A `PROCESSENTRY32W` is 568 bytes, with the process id at offset 8 and its creator's at offset 32.
    const snap = w.k32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (!snap || (snap as number) === -1 || Number(snap) >= 2 ** 63) return [];
    const entry = new Uint8Array(568);
    const view = new DataView(entry.buffer);
    const out: [number, number][] = [];
    try {
      view.setUint32(0, entry.length, true);
      for (let ok = w.k32.Process32FirstW(snap, ptr(entry)); ok; ok = w.k32.Process32NextW(snap, ptr(entry))) {
        out.push([view.getUint32(8, true), view.getUint32(32, true)]);
        view.setUint32(0, entry.length, true);
      }
    } finally {
      w.k32.CloseHandle(snap);
    }
    return out;
  }
  if (WIN) return [];
  const r = Bun.spawnSync(["ps", "-A", "-o", "pid=,ppid="], { stdout: "pipe", stderr: "ignore" });
  const out: [number, number][] = [];
  for (const line of r.stdout.toString().split("\n")) {
    const m = /^\s*(\d+)\s+(\d+)\s*$/.exec(line);
    if (m) out.push([Number(m[1]), Number(m[2])]);
  }
  return out;
}

/** Every running process, with when each started. */
export function processTable(): Proc[] {
  return ids().map(([pid, parent]) => ({ pid, parent, started: startedAt(pid) }));
}

/**
 * Every process that `root` started and that is still in `table`, nearest first. A process counts when
 * its creator is `root` or one of these, and it started no earlier than `root` and no earlier than its
 * creator; one whose start is unknown does not count. `rootStarted` is when `root` started, read while
 * it was known to be this caller's child. A process in `table` with the id `root` that started at any
 * other time means the id has been given to someone else, and nothing counts.
 */
export function descendants(root: number, rootStarted: bigint, table: Proc[]): number[] {
  const holder = table.find((p) => p.pid === root);
  if (holder !== undefined && holder.started !== rootStarted) return [];
  const children = new Map<number, Proc[]>();
  for (const p of table) if (p.pid !== p.parent) (children.get(p.parent) ?? children.set(p.parent, []).get(p.parent)!).push(p);
  const out: number[] = [];
  const seen = new Set([root]);
  for (let queue: [number, bigint][] = [[root, rootStarted]]; queue.length > 0; ) {
    const next: [number, bigint][] = [];
    for (const [pid, started] of queue) {
      for (const c of children.get(pid) ?? []) {
        if (seen.has(c.pid) || c.started === null || c.started < rootStarted || c.started < started) continue;
        seen.add(c.pid);
        out.push(c.pid);
        next.push([c.pid, c.started]);
      }
    }
    queue = next;
  }
  return out;
}

/**
 * Which of `pids` this tool may kill, and why each of the others may not: `self`, every ancestor of
 * `self` in `table`, and every process that started before `selfStarted` or whose start is unknown are
 * refused. An ancestor is found by creator ids alone, so a reused id refuses more, never less.
 */
export function screen(pids: number[], table: Proc[], self: number, selfStarted: bigint | null): { kill: number[]; refused: [number, string][] } {
  const byPid = new Map(table.map((p) => [p.pid, p]));
  const ancestors = new Set<number>();
  for (let p = byPid.get(self)?.parent; p !== undefined && p !== self && !ancestors.has(p); p = byPid.get(p)?.parent) ancestors.add(p);
  const kill: number[] = [];
  const refused: [number, string][] = [];
  for (const pid of pids) {
    const started = byPid.get(pid)?.started ?? null;
    if (pid === self) refused.push([pid, "it is this process"]);
    else if (ancestors.has(pid)) refused.push([pid, "it is an ancestor of this process"]);
    else if (started === null || selfStarted === null) refused.push([pid, "its start time cannot be read"]);
    else if (started < selfStarted) refused.push([pid, "it started before this process"]);
    else kill.push(pid);
  }
  return { kill, refused };
}

let selfStarted: bigint | null | undefined;

/** `pids` screened against the live table, with each refusal printed; the table is returned for the kill. */
function screened(pids: number[]): { kill: number[]; table: Proc[]; refused: boolean } {
  const table = processTable();
  selfStarted ??= startedAt(process.pid);
  const { kill, refused } = screen(pids, table, process.pid, selfStarted);
  for (const [pid, why] of refused) process.stderr.write(`nv: will not kill process ${pid}: ${why}\n`);
  return { kill, table, refused: refused.length > 0 };
}

// ---- a tree ----------------------------------------------------------------------------------------

/** The handle a caller owns for the child it spawned: a `Bun.Subprocess` is one. */
export interface Owned {
  readonly pid: number;
  readonly exitCode: number | null;
  readonly signalCode: string | null;
  kill(signal?: number | NodeJS.Signals): void;
}

/** Signals `own` through its handle, and only while its exit has not been observed. */
export function signalOwned(own: Owned, signal?: number | NodeJS.Signals): void {
  if (own.exitCode !== null || own.signalCode !== null) return;
  try {
    own.kill(signal);
  } catch {
    // It has exited.
  }
}

/** A process a freeze stopped, with the handle it was stopped through on Windows. */
interface Frozen {
  pid: number;
  handle: Handle | null;
}

export class Tree {
  /** A freeze walks the list until a pass finds nobody new: a process starting a child when its turn came has one the first pass missed. */
  static readonly PASSES = 5;
  /** The processes a freeze stopped, which are the ones a thaw owes. */
  private frozen: Frozen[] = [];
  private job: Handle | null = null;
  /** On Windows the child's own handle, which keeps its id from being given to another process. */
  private root: Handle | null = null;
  /** When the child started, read while it could not yet have exited. */
  private readonly started: bigint | null = null;
  readonly pid: number;

  /** Call it as soon as `Bun.spawn` returns, before anything awaits, so `own.pid` is still the child's. */
  constructor(private readonly own: Owned) {
    this.pid = own.pid;
    const w = win();
    if (w === null) {
      this.started = startedAt(this.pid);
      return;
    }
    const root = w.k32.OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SUSPEND_RESUME, 0, this.pid) as Handle | null;
    if (!root) return;
    this.root = root;
    this.started = handleStarted(root);
    const job = w.k32.CreateJobObjectW(null, null) as Handle | null;
    if (!job) return;
    if (w.k32.AssignProcessToJobObject(job, root)) this.job = job;
    else w.k32.CloseHandle(job);
  }

  /** Every live process in the tree, the root first. A process listed here may exit at any moment. */
  pids(): number[] {
    const w = win();
    if (w === null || this.job === null) return [this.pid];
    for (let room = 1024; room <= 65536; room *= 4) {
      // JOBOBJECT_BASIC_PROCESS_ID_LIST: two DWORDs, then one ULONG_PTR per process.
      const buf = new BigUint64Array(1 + room);
      const ok = w.k32.QueryInformationJobObject(this.job, JOB_PROCESS_ID_LIST, ptr(buf), buf.byteLength, null);
      if (!ok && w.k32.GetLastError() === ERROR_MORE_DATA) continue;
      if (!ok) return [this.pid];
      const listed = new Uint32Array(buf.buffer, 0, 2)[1]!;
      const found = [...buf.slice(1, 1 + listed)].map(Number);
      return found.sort((a, b) => Number(a !== this.pid) - Number(b !== this.pid));
    }
    return [this.pid];
  }

  /** Whether `pid` is a process in this tree's job right now, asked through a handle that can only query. */
  holds(pid: number): boolean {
    const w = win();
    if (w === null || this.job === null) return false;
    const handle = w.k32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) as Handle | null;
    if (!handle) return false;
    try {
      return this.inJob(handle);
    } finally {
      w.k32.CloseHandle(handle);
    }
  }

  /** Stops every process in the tree where it stands. Returns how many are stopped. */
  freeze(): number {
    if (this.frozen.length > 0) return this.frozen.length;
    const w = win();
    if (w === null) {
      if (this.own.exitCode === null && this.own.signalCode === null) {
        signalOwned(this.own, "SIGSTOP");
        this.frozen = [{ pid: this.pid, handle: null }];
      }
      return this.frozen.length;
    }
    for (let pass = 0; pass < Tree.PASSES; pass++) {
      const fresh = this.pids().filter((p) => !this.frozen.some((f) => f.pid === p));
      if (fresh.length === 0) break;
      for (const pid of fresh) {
        const handle = this.member(pid, PROCESS_SUSPEND_RESUME);
        if (handle === null) continue;
        if (w.nt.NtSuspendProcess(handle) === 0) this.frozen.push({ pid, handle });
        else if (handle !== this.root) w.k32.CloseHandle(handle);
      }
    }
    return this.frozen.length;
  }

  /** Lets every process a freeze stopped carry on, through the handle it was stopped through: the leaves first and the root last, so the agent wakes to children that already run. */
  thaw(): void {
    if (this.frozen.length === 0) return;
    const w = win();
    if (w === null) signalOwned(this.own, "SIGCONT");
    else {
      for (const { handle } of [...this.frozen].reverse()) {
        if (handle === null) continue;
        w.nt.NtResumeProcess(handle);
        if (handle !== this.root) w.k32.CloseHandle(handle);
      }
    }
    this.frozen = [];
  }

  /** Ends the whole tree, frozen or not. Safe after the child has exited: then it ends what the job still holds. */
  kill(): void {
    const w = win();
    if (w !== null) {
      if (this.job !== null) this.endJob(this.pids());
      else if (this.root !== null) w.k32.TerminateProcess(this.root, 1);
      else signalOwned(this.own, "SIGKILL");
      this.thaw();
      return;
    }
    // Linux: the tree from creator ids, read only while the child runs and is still the process it was.
    if (this.own.exitCode === null && this.own.signalCode === null && this.started !== null) {
      const { kill, table } = screened(descendants(this.pid, this.started, processTable()));
      this.endEach(kill, table);
    }
    signalOwned(this.own, "SIGKILL");
    this.frozen = [];
  }

  /** The child has exited: kills every process it started that is still in its job, and returns how many. Nothing outside Windows. */
  reap(): number {
    if (win() === null || this.job === null) return 0;
    return this.endJob(this.pids().filter((p) => p !== this.pid));
  }

  /** The tree is done with: thaws anything still frozen and gives the handles back. Kills nothing. */
  close(): void {
    this.thaw();
    const w = win();
    if (w !== null && this.job !== null) w.k32.CloseHandle(this.job);
    if (w !== null && this.root !== null) w.k32.CloseHandle(this.root);
    this.job = null;
    this.root = null;
  }

  /** Ends `members` of the job: the whole job when every one passes the screen, else each that does. */
  private endJob(members: number[]): number {
    if (members.length === 0) return 0;
    const { kill, table, refused } = screened(members);
    if (!refused) return win()!.k32.TerminateJobObject(this.job!, 1) ? kill.length : 0;
    return this.endEach(kill, table);
  }

  /** Kills each of `pids` that still started when `table` says it did and, on Windows, is still in the job. */
  private endEach(pids: number[], table: Proc[]): number {
    const started = new Map(table.map((p) => [p.pid, p.started]));
    const w = win();
    let ended = 0;
    for (const pid of pids) {
      if (w === null) {
        if (startedAt(pid) !== started.get(pid)) continue;
        try {
          process.kill(pid, "SIGKILL");
          ended++;
        } catch {
          // It has exited.
        }
        continue;
      }
      const handle = this.member(pid, PROCESS_TERMINATE);
      if (handle === null) continue;
      try {
        if (handleStarted(handle) === started.get(pid) && w.k32.TerminateProcess(handle, 1)) ended++;
      } finally {
        if (handle !== this.root) w.k32.CloseHandle(handle);
      }
    }
    return ended;
  }

  /**
   * A handle to `pid` with `access`, opened and then confirmed through that same handle to be in the job,
   * so a listed process that exited and whose id went to another process is never touched; the child's
   * own handle for the child. Null when there is no job, or the process is gone or not in it.
   */
  private member(pid: number, access: number): Handle | null {
    const w = win()!;
    if (this.job === null) return null;
    if (pid === this.pid && this.root !== null) return this.root;
    const handle = w.k32.OpenProcess(access | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) as Handle | null;
    if (!handle) return null;
    if (this.inJob(handle)) return handle;
    w.k32.CloseHandle(handle);
    return null;
  }

  private inJob(handle: Handle): boolean {
    const result = new Int32Array(1);
    return win()!.k32.IsProcessInJob(handle, this.job!, ptr(result)) !== 0 && result[0] !== 0;
  }
}
