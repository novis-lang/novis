// The Windows console that `nv` shares with every program it starts. A program attached to the same
// console may switch it into raw mode, and `wsl.exe` does, even when its own input and output are pipes.
// Two things then break for the person at the console. The input no longer turns Ctrl-C into an
// interrupt, so the key does nothing. The output no longer goes back to the first column on a bare LF,
// so every later line starts where the last one ended.
//
// `holdConsole` sets the modes a shell prompt expects, sets them again ten times a second while `nv` runs,
// and sets them once more as it exits, so the shell it returns to gets them too. The modes are fixed bits
// rather than the ones found at start, so an `nv` started by another `nv` holds the same ones. None of
// this runs off Windows, when no standard stream is a console, or when the console cannot be opened.

import { dlopen, FFIType, ptr } from "bun:ffi";

/** Ctrl-C is an interrupt, input is read a line at a time, and what is typed is echoed. */
const INPUT_ON = 0x0001 | 0x0002 | 0x0004;
/** Keys arrive as escape sequences. */
const INPUT_OFF = 0x0200;
/** Control characters are acted on, a long line wraps, and escape sequences are acted on. */
const OUTPUT_ON = 0x0001 | 0x0002 | 0x0004;
/** A bare LF moves down a row without going back to the first column. */
const OUTPUT_OFF = 0x0008;

const INTERVAL_MS = 100;

interface Handles {
  api: ReturnType<typeof load>;
  input: number | null;
  output: number | null;
}

let handles: Handles | null | undefined;
let timer: ReturnType<typeof setInterval> | null = null;

function load() {
  return dlopen("kernel32.dll", {
    CreateFileW: { args: [FFIType.ptr, FFIType.u32, FFIType.u32, FFIType.ptr, FFIType.u32, FFIType.u32, FFIType.ptr], returns: FFIType.i64_fast },
    GetConsoleMode: { args: [FFIType.i64, FFIType.ptr], returns: FFIType.i32 },
    SetConsoleMode: { args: [FFIType.i64, FFIType.u32], returns: FFIType.i32 },
  }).symbols;
}

/** The console's input or output buffer, or null when the process has no console. */
function open(api: Handles["api"], name: string): number | null {
  // GENERIC_READ | GENERIC_WRITE, shared for reading and writing, OPEN_EXISTING.
  const h = Number(api.CreateFileW(ptr(Buffer.from(`${name}\0`, "utf16le")), 0xc0000000, 3, null, 3, 0, null));
  if (h === -1 || h === 0) return null;
  const mode = new Uint32Array(1);
  return api.GetConsoleMode(h, ptr(mode)) ? h : null;
}

function opened(): Handles | null {
  if (handles !== undefined) return handles;
  handles = null;
  if (process.platform !== "win32") return null;
  try {
    const api = load();
    const found = { api, input: open(api, "CONIN$"), output: open(api, "CONOUT$") };
    if (found.input !== null || found.output !== null) handles = found;
  } catch {
    handles = null;
  }
  return handles;
}

function force(api: Handles["api"], h: number, on: number, off: number): boolean {
  const mode = new Uint32Array(1);
  if (!api.GetConsoleMode(h, ptr(mode))) return false;
  const want = (mode[0]! | on) & ~off;
  return want === mode[0] || api.SetConsoleMode(h, want) !== 0;
}

/**
 * Sets the console's modes now. Returns whether the output buffer takes escape sequences, which is false
 * off Windows and with no console, since there the caller has no mode to depend on.
 */
export function cookConsole(): boolean {
  const h = opened();
  if (h === null) return false;
  if (h.input !== null) force(h.api, h.input, INPUT_ON, INPUT_OFF);
  return h.output !== null && force(h.api, h.output, OUTPUT_ON, OUTPUT_OFF);
}

/** Keeps the console's modes set for as long as this process runs; see the module doc. */
export function holdConsole(): void {
  if (timer !== null || process.platform !== "win32") return;
  if (!(process.stdin.isTTY || process.stdout.isTTY || process.stderr.isTTY)) return;
  if (opened() === null) return;
  cookConsole();
  timer = setInterval(cookConsole, INTERVAL_MS);
  timer.unref();
  process.on("exit", cookConsole);
}
