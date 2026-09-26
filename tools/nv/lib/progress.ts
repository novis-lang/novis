// What a long `nv` command is doing while it does it: a build, a roster read, a pool of programs. The
// text never goes into what the command prints, because an agent reads that output and pays for every
// line of it. It goes to one of two places, or to none:
//
//   - with `NV_PROGRESS_DIR` set, to `<dir>/<pid>.json`, which the loop driver sets for every session and
//     check it starts. `readProgress` reads the directory back, and `nv loop` puts the text on its status
//     row after the session's current tool call. The file is written at most four times a second and is
//     deleted when the command exits; a file whose process is gone is deleted by the next read;
//   - otherwise, with stderr a terminal, to one line on stderr that is rewritten in place and ends in how
//     long the command has run. Any write to stdout or stderr erases the line first, so the output a person
//     reads is the same as without it. The next `progress` call paints the line again;
//   - otherwise, which is every shell call an agent makes, nowhere.
//
// The driver's own console paints its status block on the same terminal, so under `NV_PROGRESS_DIR`
// nothing is painted even when stderr is a terminal.

import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** The environment variable that names the directory progress files go to. */
export const ENV = "NV_PROGRESS_DIR";

const FILE_EVERY_MS = 250;
const LINE_EVERY_MS = 100;
const TICK_MS = 1000;

const started = performance.now();
let text = "";
let fileAt = 0;
let fileTimer: ReturnType<typeof setTimeout> | null = null;
let filePath: string | null = null;
let lineAt = 0;
let lineTimer: ReturnType<typeof setTimeout> | null = null;
let ticker: ReturnType<typeof setInterval> | null = null;
/** Whether the terminal line is on screen now. */
let showing = false;
let hooked = false;

const rawErr = process.stderr.write.bind(process.stderr);

function mode(): "file" | "line" | "none" {
  if (process.env[ENV]) return "file";
  return process.stderr.isTTY ? "line" : "none";
}

/** `m:ss` since the command started. */
function elapsed(): string {
  const s = Math.floor((performance.now() - started) / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** Says what the command is doing now. Each call replaces the text of the one before it. */
export function progress(now: string): void {
  text = now.replace(/[\r\n]+/g, " ");
  const m = mode();
  if (m === "file") writeSoon();
  else if (m === "line") paintSoon();
}

/** Erases the terminal line and removes the progress file. The command is still running, and its next
 * `progress` call shows it again. */
export function clear(): void {
  erase();
  if (filePath !== null) rmSync(filePath, { force: true });
  filePath = null;
}

function writeSoon(): void {
  if (fileTimer !== null) return;
  const wait = Math.max(0, fileAt + FILE_EVERY_MS - performance.now());
  if (wait > 0) {
    fileTimer = setTimeout(writeFile, wait);
    fileTimer.unref();
  } else writeFile();
}

function writeFile(): void {
  fileTimer = null;
  fileAt = performance.now();
  const dir = process.env[ENV];
  if (!dir) return;
  if (filePath === null) {
    mkdirSync(dir, { recursive: true });
    filePath = join(dir, `${process.pid}.json`);
    process.on("exit", clear);
  }
  try {
    writeFileSync(filePath, JSON.stringify({ pid: process.pid, started: Date.now() - Math.round(performance.now() - started), text }));
  } catch {
    // A progress file that cannot be written is only a status line that says less.
  }
}

function paintSoon(): void {
  hook();
  showing = true;
  if (lineTimer !== null) return;
  const wait = Math.max(0, lineAt + LINE_EVERY_MS - performance.now());
  if (wait > 0) {
    lineTimer = setTimeout(paint, wait);
    lineTimer.unref();
  } else paint();
}

function paint(): void {
  lineTimer = null;
  if (!showing) return;
  lineAt = performance.now();
  const clock = ` ${elapsed()}`;
  const room = Math.max(10, (process.stderr.columns ?? 100) - 1 - clock.length);
  const chars = Array.from(text);
  const shown = chars.length <= room ? text : `${chars.slice(0, room - 1).join("")}…`;
  // The cursor goes back to the first column, so even a write that does not erase starts there.
  rawErr(`\r${shown}${clock}\x1b[K\r`);
}

/** Erases the terminal line, for a child that writes to the same terminal. The next `progress` call paints
 * it again. */
export function erase(): void {
  if (!showing) return;
  showing = false;
  rawErr("\r\x1b[K");
}

/** Makes every write to stdout and stderr erase the line first, and keeps the clock moving. */
function hook(): void {
  if (hooked) return;
  hooked = true;
  for (const stream of [process.stdout, process.stderr]) {
    const write = stream.write.bind(stream) as (...a: unknown[]) => boolean;
    stream.write = ((...a: unknown[]) => {
      erase();
      return write(...a);
    }) as typeof stream.write;
  }
  // Bun's console writes to the file descriptors directly, not through `stream.write`.
  for (const name of ["log", "info", "warn", "error", "debug"] as const) {
    const f = console[name].bind(console);
    console[name] = (...a: unknown[]) => {
      erase();
      f(...a);
    };
  }
  ticker = setInterval(() => showing && paint(), TICK_MS);
  ticker.unref();
  process.on("exit", erase);
}

/**
 * Reads cargo's status lines one at a time and returns what each one says, `compiling nvs-stdlib (43
 * crates)`, or null for any other line. cargo writes those lines to stderr whether or not stderr is a
 * terminal, and beside `--message-format=json` too.
 */
export function cargoStatus(): (line: string) => string | null {
  let units = 0;
  return (line) => {
    const m = /^\s*(Compiling|Checking|Documenting|Running|Finished|Blocking|Updating|Downloading|Downloaded|Locking)\s+(.*)$/.exec(line);
    if (!m) return null;
    const verb = m[1]!;
    const what = m[2]!.split(" ")[0] ?? "";
    if (verb === "Compiling" || verb === "Checking" || verb === "Documenting") units++;
    if (verb === "Finished") return "finished";
    return `${verb.toLowerCase()} ${what}${units ? ` (${units} crate${units === 1 ? "" : "s"})` : ""}`;
  };
}

/** A `proc.run` `onLine` that shows cargo's status lines as progress: `<label>: compiling nvs-stdlib (43 crates)`. */
export function cargoLines(label: string): (line: string) => void {
  const status = cargoStatus();
  return (line) => {
    const said = status(line);
    if (said !== null) progress(`${label}: ${said}`);
  };
}

interface Entry {
  pid: number;
  started: number;
  text: string;
}

function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return (e as NodeJS.ErrnoException).code === "EPERM";
  }
}

/** What every running `nv` command under `dir` is doing, the one started first first. A file whose
 * process has exited is deleted. */
export function readProgress(dir: string): string[] {
  let names: string[];
  try {
    names = readdirSync(dir).filter((n) => n.endsWith(".json"));
  } catch {
    return [];
  }
  const found: Entry[] = [];
  for (const name of names) {
    const path = join(dir, name);
    let e: Entry;
    try {
      e = JSON.parse(readFileSync(path, "utf8")) as Entry;
    } catch {
      // Read while it was being written; the next read gets it whole.
      continue;
    }
    if (!alive(e.pid)) {
      rmSync(path, { force: true });
      continue;
    }
    if (e.text) found.push(e);
  }
  return found.sort((a, b) => a.started - b.started).map((e) => e.text);
}
