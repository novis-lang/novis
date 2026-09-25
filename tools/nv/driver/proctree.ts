// A child process and everything it started, as one thing: freeze it, thaw it, kill it. `nv loop` halts a
// running session with it, and a halted session has to stop all of what it runs, since a frozen agent
// whose `cargo build` keeps writing `target/` is not halted. Carrying on resumes every process at the
// instruction it stopped on, so nothing is lost and nothing is sent again.
//
// On Windows the child is put in a Job Object right after it starts. Every process it starts from then on
// is in the job and cannot leave it, so the job's process list is the tree, including a grandchild whose
// parent has already exited. Each process is stopped with `NtSuspendProcess`. The job has no limits, so
// closing it at the end of a session kills nothing. Elsewhere the child alone gets `SIGSTOP` and
// `SIGCONT`, since `Bun.spawn` cannot start it in a process group of its own.
//
// Every method is best effort and never throws: a freeze that cannot be taken shows in its count, and
// must not end a run.

import { dlopen, FFIType, ptr, type Pointer } from "bun:ffi";

/** A Windows handle as the FFI hands it back. */
type Handle = Pointer;

const WIN = process.platform === "win32";
const PROCESS_TERMINATE = 0x0001;
const PROCESS_SET_QUOTA = 0x0100;
const PROCESS_QUERY_INFORMATION = 0x0400;
const PROCESS_SUSPEND_RESUME = 0x0800;
const JOB_PROCESS_ID_LIST = 3;
const ERROR_MORE_DATA = 234;

type Api = ReturnType<typeof load>;
let api: Api | null | undefined;

function load() {
  const k32 = dlopen("kernel32.dll", {
    CreateJobObjectW: { args: [FFIType.ptr, FFIType.ptr], returns: FFIType.ptr },
    OpenProcess: { args: [FFIType.u32, FFIType.i32, FFIType.u32], returns: FFIType.ptr },
    AssignProcessToJobObject: { args: [FFIType.ptr, FFIType.ptr], returns: FFIType.i32 },
    QueryInformationJobObject: { args: [FFIType.ptr, FFIType.i32, FFIType.ptr, FFIType.u32, FFIType.ptr], returns: FFIType.i32 },
    TerminateJobObject: { args: [FFIType.ptr, FFIType.u32], returns: FFIType.i32 },
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

export class Tree {
  /** A freeze walks the list until a pass finds nobody new: a process starting a child when its turn came has one the first pass missed. */
  static readonly PASSES = 5;
  /** The pids a freeze stopped, which are the ones a thaw owes. */
  private frozen: number[] = [];
  private job: Handle | null = null;

  constructor(readonly pid: number) {
    const w = win();
    if (w === null) return;
    const job = w.k32.CreateJobObjectW(null, null) as Handle | null;
    if (!job) return;
    const handle = w.k32.OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_QUERY_INFORMATION, 0, pid) as Handle | null;
    if (handle && w.k32.AssignProcessToJobObject(job, handle)) this.job = job;
    else w.k32.CloseHandle(job);
    if (handle) w.k32.CloseHandle(handle);
  }

  /** Every live process in the tree, the root first. */
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

  /** Stops every process in the tree where it stands. Returns how many are stopped. */
  freeze(): number {
    if (this.frozen.length > 0) return this.frozen.length;
    const w = win();
    if (w === null) {
      try {
        process.kill(this.pid, "SIGSTOP");
        this.frozen = [this.pid];
      } catch {
        // It has exited.
      }
      return this.frozen.length;
    }
    for (let pass = 0; pass < Tree.PASSES; pass++) {
      const fresh = this.pids().filter((p) => !this.frozen.includes(p));
      if (fresh.length === 0) break;
      for (const pid of fresh) if (this.each(pid, w.nt.NtSuspendProcess)) this.frozen.push(pid);
    }
    return this.frozen.length;
  }

  /** Lets every process a freeze stopped carry on: the leaves first and the root last, so the agent wakes to children that already run. */
  thaw(): void {
    if (this.frozen.length === 0) return;
    const w = win();
    if (w === null) {
      try {
        process.kill(this.pid, "SIGCONT");
      } catch {
        // It has exited.
      }
    } else {
      for (const pid of [...this.frozen].reverse()) this.each(pid, w.nt.NtResumeProcess);
    }
    this.frozen = [];
  }

  /** Ends the whole tree, frozen or not. */
  kill(): void {
    const w = win();
    try {
      if (w !== null && this.job !== null) w.k32.TerminateJobObject(this.job, 1);
      else process.kill(this.pid, "SIGKILL");
    } catch {
      // It has exited.
    }
    this.frozen = [];
  }

  /** The session is over: thaws anything still frozen and gives the job handle back. Kills nothing. */
  close(): void {
    this.thaw();
    const w = win();
    if (w !== null && this.job !== null) w.k32.CloseHandle(this.job);
    this.job = null;
  }

  private each(pid: number, call: (h: Handle) => number): boolean {
    const w = win()!;
    const handle = w.k32.OpenProcess(PROCESS_SUSPEND_RESUME, 0, pid) as Handle | null;
    // A process that exited between the list and now, or one that is not ours to touch.
    if (!handle) return false;
    try {
      return call(handle) === 0;
    } finally {
      w.k32.CloseHandle(handle);
    }
  }
}
