#!/usr/bin/env python3
"""A child process and everything it started, as one thing: freeze it, thaw it, kill it.

`tools/loop.py` runs a `claude` session that runs `cargo`, `python`, shells and MCP servers under
itself. Halting a session means stopping ALL of that where it stands -- a frozen agent whose
`cargo build` keeps writing `target/` is not halted -- and carrying on means every one of them
picking up mid-instruction, with nothing lost and nothing re-sent. That is a suspend, not a kill.

    tree = proctree.Tree(subprocess.Popen(cmd, **proctree.popen_kwargs(), ...))
    tree.freeze()      # how many processes were stopped
    tree.thaw()
    tree.kill()        # the whole tree, frozen or not

How the tree is known, which is the whole difficulty:

* **Windows**: the child is put in a Job Object the moment it exists. Every process it starts
  from then on is in the job too and cannot leave it, so the job's process list IS the tree --
  exactly, including a grandchild whose parent has already exited, which a walk over parent ids
  loses. Each one is stopped with `NtSuspendProcess`. The job is given no limits, so closing it
  at the end of a session kills nothing.
* **POSIX**: the child is started in a session of its own (`popen_kwargs`), so its process group
  is the tree, and `SIGSTOP` / `SIGCONT` / `SIGKILL` go to the group. A terminal's Ctrl-C does
  not reach a group the terminal is not the controller of, so whoever owns a `Tree` kills it on
  the way out.

Standard library only, like everything the loop runs on.
"""

from __future__ import annotations

import os
import signal
import subprocess

IS_WINDOWS = os.name == "nt"

if IS_WINDOWS:
    import ctypes
    from ctypes import wintypes

    _k32 = ctypes.WinDLL("kernel32", use_last_error=True)
    _nt = ctypes.WinDLL("ntdll")
    _k32.CreateJobObjectW.restype = wintypes.HANDLE
    _k32.CreateJobObjectW.argtypes = [wintypes.LPVOID, wintypes.LPCWSTR]
    _k32.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
    _k32.QueryInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int, wintypes.LPVOID,
                                               wintypes.DWORD, wintypes.LPDWORD]
    _k32.TerminateJobObject.argtypes = [wintypes.HANDLE, wintypes.UINT]
    _k32.OpenProcess.restype = wintypes.HANDLE
    _k32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    _k32.CloseHandle.argtypes = [wintypes.HANDLE]
    _nt.NtSuspendProcess.argtypes = [wintypes.HANDLE]
    _nt.NtResumeProcess.argtypes = [wintypes.HANDLE]

    _PROCESS_SUSPEND_RESUME = 0x0800
    _JOB_PROCESS_ID_LIST = 3  # JobObjectBasicProcessIdList


def popen_kwargs():
    """What `subprocess.Popen` must be given for the child's tree to be findable afterwards."""
    return {} if IS_WINDOWS else {"start_new_session": True}


class Tree:
    """`proc` and every process under it. Every method is best effort and never raises: a halt
    that cannot be taken is reported by its return value, and must not take a run down."""

    #: A freeze walks the list until a pass finds nobody new, because a process that was starting
    #: a child when its turn came has one the first pass never saw. Bounded, so a tree that keeps
    #: growing under a frozen root -- which it cannot -- could not spin here.
    PASSES = 5

    def __init__(self, proc: subprocess.Popen):
        self.proc = proc
        self.frozen: list[int] = []  # the pids a freeze stopped, which are the ones a thaw owes
        self.job = None
        if IS_WINDOWS:
            try:
                job = _k32.CreateJobObjectW(None, None)
                if job and _k32.AssignProcessToJobObject(job, wintypes.HANDLE(int(proc._handle))):
                    self.job = job
                elif job:
                    _k32.CloseHandle(job)
            except (OSError, AttributeError, ValueError):
                self.job = None

    # -- who is in it ------------------------------------------------------------------

    def pids(self):
        """Every live process in the tree, the root first."""
        if not IS_WINDOWS or not self.job:
            return [self.proc.pid] if self.proc.poll() is None else []
        room = 1024
        while True:
            class IdList(ctypes.Structure):
                _fields_ = [("assigned", wintypes.DWORD), ("listed", wintypes.DWORD),
                            ("ids", ctypes.c_size_t * room)]
            ids = IdList()
            ok = _k32.QueryInformationJobObject(self.job, _JOB_PROCESS_ID_LIST,
                                                ctypes.byref(ids), ctypes.sizeof(ids), None)
            if not ok and ctypes.get_last_error() == 234 and room < 65536:  # ERROR_MORE_DATA
                room *= 4
                continue
            if not ok:
                return [self.proc.pid] if self.proc.poll() is None else []
            found = [int(p) for p in ids.ids[: ids.listed]]
            return sorted(found, key=lambda p: p != self.proc.pid)

    # -- freeze and thaw ---------------------------------------------------------------

    def freeze(self):
        """Stop every process in the tree where it stands. Returns how many were stopped."""
        if self.frozen:
            return len(self.frozen)
        if not IS_WINDOWS:
            try:
                os.killpg(os.getpgid(self.proc.pid), signal.SIGSTOP)
                self.frozen = [self.proc.pid]
            except (OSError, ProcessLookupError):
                pass
            return len(self.frozen)
        for _ in range(self.PASSES):
            fresh = [p for p in self.pids() if p not in self.frozen]
            if not fresh:
                break
            for pid in fresh:
                if self._each(pid, _nt.NtSuspendProcess):
                    self.frozen.append(pid)
        return len(self.frozen)

    def thaw(self):
        """Let every process a freeze stopped carry on. Leaves first and the root last, so the
        agent wakes up to children that are already running again."""
        if not self.frozen:
            return
        if not IS_WINDOWS:
            try:
                os.killpg(os.getpgid(self.proc.pid), signal.SIGCONT)
            except (OSError, ProcessLookupError):
                pass
        else:
            for pid in reversed(self.frozen):
                self._each(pid, _nt.NtResumeProcess)
        self.frozen = []

    def _each(self, pid, call):
        handle = _k32.OpenProcess(_PROCESS_SUSPEND_RESUME, False, pid)
        if not handle:
            return False  # it exited between the list and now, or it is not ours to touch
        try:
            return call(handle) == 0
        finally:
            _k32.CloseHandle(handle)

    # -- the way out -------------------------------------------------------------------

    def kill(self):
        """End the whole tree, frozen or not."""
        try:
            if IS_WINDOWS and self.job:
                _k32.TerminateJobObject(self.job, 1)
            elif not IS_WINDOWS:
                os.killpg(os.getpgid(self.proc.pid), signal.SIGKILL)
        except (OSError, ProcessLookupError):
            pass
        try:
            self.proc.kill()
        except OSError:
            pass
        self.frozen = []

    def close(self):
        """The session is over: thaw anything still frozen, and give the job handle back. Kills
        nothing -- a process the session left running on purpose stays running."""
        self.thaw()
        if IS_WINDOWS and self.job:
            _k32.CloseHandle(self.job)
            self.job = None


if __name__ == "__main__":
    import sys

    if {"-h", "--help"} & set(sys.argv[1:]):
        print(__doc__)
        raise SystemExit(0)
    sys.stderr.write("proctree.py is a module -- `tools/loop.py` imports it.\n")
    raise SystemExit(2)
