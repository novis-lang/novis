// The program the launch tests run in place of a project's server, as `bun launch-stand-in.ts <mode>`.
// It ports looper-job's `stand-in` and `window-stand-in`.
//
// - `serve <mark> <ms> <code> <exit|ignore> [line …]` prints `exe <path>` once its Ctrl+Break handler is
//   set, then each line. A line `@<ms>` waits that long instead, and a line `:<port>` listens on that
//   TCP port of `127.0.0.1` instead. It then runs for `ms` and exits with
//   `code`. A Ctrl+Break writes the file `mark`, and then exits 0 with `exit` or is ignored with `ignore`.
// - `window <x> <y> <ms>` prints `start <flags> <show>` from its start options, opens one window at `x`,
//   `y`, serves its messages for `ms`, and prints `end <x> <y>` with where the window stood last while
//   it was not minimized. So a window that something outside the test minimizes still reports where
//   the watcher put it.

import { writeFileSync } from "node:fs";
import { dlopen, FFIType, ptr } from "bun:ffi";

const [mode, ...args] = process.argv.slice(2);

/** Writes `line` to stdout at once, since a pipe would otherwise hold it. */
function say(line: string): void {
  process.stdout.write(`${line}\n`);
}

async function serve([mark, ms, code, onBreak, ...lines]: string[]): Promise<never> {
  process.on("SIGBREAK", () => {
    writeFileSync(mark as string, "asked");
    if (onBreak === "exit") process.exit(0);
  });
  say(`exe ${process.execPath}`);
  for (const line of lines) {
    if (line.startsWith("@")) await Bun.sleep(Number(line.slice(1)));
    else if (line.startsWith(":")) Bun.listen({ hostname: "127.0.0.1", port: Number(line.slice(1)), socket: { data() {} } });
    else say(line);
  }
  await Bun.sleep(Number(ms));
  process.exit(Number(code));
}

async function window([x, y, ms]: string[]): Promise<never> {
  const { ptr: P, i32, u32 } = FFIType;
  const k = dlopen("kernel32.dll", {
    GetStartupInfoW: { args: [P], returns: FFIType.void },
    GetModuleHandleW: { args: [P], returns: P },
  }).symbols;
  const u = dlopen("user32.dll", {
    CreateWindowExW: { args: [u32, P, P, u32, i32, i32, i32, i32, P, P, P, P], returns: P },
    ShowWindow: { args: [P, i32], returns: i32 },
    GetWindowRect: { args: [P, P], returns: i32 },
    IsIconic: { args: [P], returns: i32 },
    PeekMessageW: { args: [P, P, u32, u32, u32], returns: i32 },
    TranslateMessage: { args: [P], returns: i32 },
    DispatchMessageW: { args: [P], returns: FFIType.i64 },
  }).symbols;
  const info = Buffer.alloc(104);
  info.writeUInt32LE(104, 0);
  k.GetStartupInfoW(ptr(info));
  say(`start ${info.readUInt32LE(60)} ${info.readUInt16LE(64)}`);
  const wide = (s: string) => Buffer.from(`${s}\0`, "utf16le");
  const WS_OVERLAPPEDWINDOW = 0x00cf0000;
  const SW_SHOWDEFAULT = 10;
  const hwnd = u.CreateWindowExW(0, ptr(wide("STATIC")), ptr(wide("looper stand-in")), WS_OVERLAPPEDWINDOW, Number(x), Number(y), 300, 200, null, null, k.GetModuleHandleW(null), null);
  if (!hwnd) throw new Error("cannot create the window");
  u.ShowWindow(hwnd, SW_SHOWDEFAULT);
  const msg = Buffer.alloc(48);
  const rect = Buffer.alloc(16);
  let corner = [Number(x), Number(y)];
  const look = (): void => {
    if (u.IsIconic(hwnd) === 0 && u.GetWindowRect(hwnd, ptr(rect)) !== 0) corner = [rect.readInt32LE(0), rect.readInt32LE(4)];
  };
  const until = Date.now() + Number(ms);
  while (Date.now() < until) {
    while (u.PeekMessageW(ptr(msg), null, 0, 0, 1) !== 0) {
      u.TranslateMessage(ptr(msg));
      u.DispatchMessageW(ptr(msg));
    }
    look();
    await Bun.sleep(10);
  }
  look();
  say(`end ${corner[0]} ${corner[1]}`);
  process.exit(0);
}

if (mode === "serve") await serve(args);
else if (mode === "window") await window(args);
else throw new Error(`launch-stand-in takes serve or window, and got ${mode}`);
