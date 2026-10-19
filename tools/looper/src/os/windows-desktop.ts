// The Windows calls of the desktop: keeping a job's top-level windows off the screen, and telling
// whether a job shows one. They go to `user32` through `bun:ffi`. It ports
// `crates/looper-job/src/windows.rs`, and `windows.ts` sets the start options that go with it.
//
// - `watchWindows` hooks the show events of the whole desktop on this thread, and serves them from a
//   timer every `PUMP_MS`. It keeps the windows of the job's processes, and moves each visible top-level
//   window to `OFF`. It also polls the job's windows every `POLL_MS`, which catches a window that moved
//   back and any event it missed.
// - A window is never hidden or minimized, since such a window may stop drawing, and a minimized window
//   is left where it is. A move is posted to the window's own thread, so a hung program never blocks the
//   watcher.
// - `showsWindow` says whether a job shows any visible top-level window, on the screen or off it.
//
// The Rust watcher also hooks the moves of every window. This one does not, because each move of any
// window, the cursor's too, would call into JavaScript. A window that moves back stays on the screen
// until the next poll.

import { dlopen, FFIType, JSCallback, type Pointer, ptr } from "bun:ffi";

/** Where a hidden window's top-left corner goes. */
export const OFF: readonly [number, number] = [-20_000, -20_000];
/** A window whose left and top are both at or below this counts as off the screen already. */
export const FAR = -10_000;
/** How often a watcher serves the events it hooked. */
export const PUMP_MS = 25;
/** How often a watcher polls the job's windows. */
export const POLL_MS = 250;

const GA_ROOT = 2;
const EVENT_OBJECT_SHOW = 0x8002;
const WINEVENT_SKIPOWNPROCESS = 0x2;
const OBJID_WINDOW = 0;
const PM_REMOVE = 1;
const SWP_NOSIZE = 0x1;
const SWP_NOZORDER = 0x4;
const SWP_NOACTIVATE = 0x10;
const SWP_NOOWNERZORDER = 0x200;
const SWP_ASYNCWINDOWPOS = 0x4000;
const MOVE = SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS;

const { ptr: P, i32, u32, i64 } = FFIType;

let user: ReturnType<typeof open> | undefined;

function open() {
  return dlopen("user32.dll", {
    EnumWindows: { args: [P, i64], returns: i32 },
    GetWindowThreadProcessId: { args: [P, P], returns: u32 },
    GetAncestor: { args: [P, u32], returns: P },
    IsWindowVisible: { args: [P], returns: i32 },
    IsIconic: { args: [P], returns: i32 },
    GetWindowRect: { args: [P, P], returns: i32 },
    SetWindowPos: { args: [P, P, i32, i32, i32, i32, u32], returns: i32 },
    SetWinEventHook: { args: [u32, u32, P, P, u32, u32, u32], returns: P },
    UnhookWinEvent: { args: [P], returns: i32 },
    PeekMessageW: { args: [P, P, u32, u32, u32], returns: i32 },
    DispatchMessageW: { args: [P], returns: i64 },
  }).symbols;
}

/** The library, opened on first use. */
function lib() {
  user ??= open();
  return user;
}

/** The pid of the process that owns `hwnd`, 0 when the window is gone. */
function owner(hwnd: Pointer): number {
  const pid = Buffer.alloc(4);
  lib().GetWindowThreadProcessId(hwnd, ptr(pid));
  return pid.readUInt32LE(0);
}

/** Whether `hwnd` is a visible top-level window. */
function topVisible(hwnd: Pointer): boolean {
  return lib().GetAncestor(hwnd, GA_ROOT) === hwnd && lib().IsWindowVisible(hwnd) !== 0;
}

/** Calls `each` with every top-level window, until it gives false. */
function enumWindows(each: (hwnd: Pointer) => boolean): void {
  const cb = new JSCallback((hwnd: Pointer) => (each(hwnd) ? 1 : 0), { args: [P, i64], returns: i32 });
  try {
    lib().EnumWindows(cb.ptr, 0n);
  } finally {
    cb.close();
  }
}

/** Whether any of `pids` shows a visible top-level window now, on the screen or off it. */
export function showsWindow(pids: readonly number[]): boolean {
  if (pids.length === 0) return false;
  let found = false;
  enumWindows((hwnd) => {
    found = pids.includes(owner(hwnd)) && topVisible(hwnd);
    return !found;
  });
  return found;
}

/** A watcher that keeps one job's windows off the screen until it is stopped. */
export interface WindowWatcher {
  /** How many moves it has asked for. */
  moved(): number;
  stop(): void;
}

/** Starts keeping off the screen the windows of the processes `pids` gives, which it reads again as they change. */
export function watchWindows(pids: () => number[]): WindowWatcher {
  const u = lib();
  let known: number[] = [];
  let moved = 0;
  const place = (hwnd: Pointer): void => {
    if (!topVisible(hwnd) || u.IsIconic(hwnd) !== 0) return;
    const rect = Buffer.alloc(16);
    if (u.GetWindowRect(hwnd, ptr(rect)) === 0) return;
    if (rect.readInt32LE(0) <= FAR && rect.readInt32LE(4) <= FAR) return;
    if (u.SetWindowPos(hwnd, null, OFF[0], OFF[1], 0, 0, MOVE) !== 0) moved++;
  };
  const owns = (hwnd: Pointer): boolean => {
    const pid = owner(hwnd);
    return pid !== 0 && known.includes(pid);
  };
  const poll = (): void => {
    known = pids();
    if (known.length > 0) enumWindows((hwnd) => (owns(hwnd) && place(hwnd), true));
  };
  const onShow = new JSCallback(
    (_hook: Pointer, _event: number, hwnd: Pointer | null, object: number, child: number) => {
      if (!hwnd || object !== OBJID_WINDOW || child !== 0) return;
      // A window shown by a process that started since the last poll.
      if (!owns(hwnd)) known = pids();
      if (owns(hwnd)) place(hwnd);
    },
    { args: [P, u32, P, i32, i32, u32, u32], returns: FFIType.void },
  );
  const hook = u.SetWinEventHook(EVENT_OBJECT_SHOW, EVENT_OBJECT_SHOW, null, onShow.ptr, 0, 0, WINEVENT_SKIPOWNPROCESS);
  const msg = Buffer.alloc(48);
  let polled = 0;
  const timer = setInterval(() => {
    // Serving the queue runs the hook's callback for each show event since the last tick.
    while (u.PeekMessageW(ptr(msg), null, 0, 0, PM_REMOVE) !== 0) u.DispatchMessageW(ptr(msg));
    if (Date.now() - polled >= POLL_MS) {
      polled = Date.now();
      poll();
    }
  }, PUMP_MS);
  poll();
  polled = Date.now();
  return {
    moved: () => moved,
    stop: () => {
      clearInterval(timer);
      if (hook) u.UnhookWinEvent(hook);
      onShow.close();
    },
  };
}
