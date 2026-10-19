// A loop's controls: each one a line appended to `controls.ndjson` in the loop's folder, which the
// worker reads before each turn and while a session runs (decision 0027). It ports the control rows of
// `crates/looper-store/src/query.rs`, with a file where looper has a store table.
//
// - **A line is one `Control`:** `{"at", "by", "action", "payload"}`, with `refused` when the asker
//   was refused. `by` is `ui`, `cli:<who>`, `agent` or `looper`. Any process appends one with
//   `appendControl`, under the loop's named mutex, as one write call with its newline. A file whose
//   last line a killed writer tore is ended with a newline first, so the torn line is skipped alone.
// - **The pause that stands** is folded from the lines in order (`standingPause`): a `start`, and a
//   `resume` its asker `canLift`, lift it, and a `pause` sets its owner as `pauseOwner` says. So a
//   worker that starts finds a person's pause still standing, and a `start` always starts it running.
// - **A `resume` that cannot lift the standing pause is refused at the append.** The line is written with
//   `refused` naming why, so the ask stays on record, and `appendControl` throws that reason. The fold
//   and the worker pass over a refused line.
// - **A `ControlReader` reads the lines past its offset,** complete lines only. A line that does not
//   parse, or is longer than `LINE_BYTES`, is skipped.
//
// # The bounds it keeps
//
// - The file is compacted by its worker once it has read past `CONTROLS_BYTES` (`compact`). Under the
//   mutex, the file is replaced by one `compacted` line that carries the standing pause, so the fold
//   gives the same owner and reason after it.
// - One read takes at most `READ_BYTES`, and a line at most `LINE_BYTES`. A longer one is refused on
//   append.

import { closeSync, existsSync, fstatSync, mkdirSync, openSync, readSync, statSync, writeSync } from "node:fs";
import { join } from "node:path";
import { Refused } from "../goal/model.ts";
import { type Mutex, openMutex } from "../os/index.ts";
import { writeState } from "../store/state.ts";

/** The controls file in a loop's folder. */
export const CONTROLS_FILE = "controls.ndjson";
/** The size past which the worker compacts the file once it has read it all. */
export const CONTROLS_BYTES = 256 * 1024;
/** The longest line a control may take, newline included. */
export const LINE_BYTES = 64 * 1024;
/** The most bytes one read takes. */
export const READ_BYTES = 1024 * 1024;
/** How often a wait on the mutex tries it again, in ms. */
const LOCK_EVERY_MS = 5;

/** Every action a control may name. `warn` and `compacted` are the worker's own records. */
export const ACTIONS = [
  "start",
  "stop",
  "stop-now",
  "pause",
  "resume",
  "freeze",
  "thaw",
  "say",
  "interrupt",
  "retry-now",
  "skip-stage",
  "check",
  "set",
  "warn",
  "compacted",
] as const;

/** A pause that stands: who owns it, and why, which is the payload of the newest pause its owner asked for. */
export interface Pause {
  by: string;
  why: string;
}

/** One line of the controls file. */
export interface Control {
  /** In ms since the Unix epoch. */
  at: number;
  by: string;
  action: string;
  payload: string;
  /** Why the asker was refused. The worker passes over a refused control. */
  refused?: string;
  /** On a `compacted` line, the pause that stood when the file was compacted. */
  pause?: Pause | null;
}

/** Whether `by` is a person: `ui` or `cli:<who>`, and not `agent` or `looper`. */
export function isPerson(by: string): boolean {
  return by === "ui" || by.startsWith("cli:");
}

/** Whether `asker` may lift a pause `owner` made. An `agent` cannot lift a person's pause. Anyone may lift `looper`'s or an agent's. */
export function canLift(asker: string, owner: string): boolean {
  return asker !== "agent" || !isPerson(owner);
}

/** Who owns a pause `owner` holds once `by` asks to pause again: the first asker, unless `by` is a person, who takes it over. */
export function pauseOwner(owner: string | null, by: string): string {
  return owner !== null && !isPerson(by) ? owner : by;
}

/** The pause that stands after `controls`, in order, or null when none stands. */
export function standingPause(controls: Control[]): Pause | null {
  let pause: Pause | null = null;
  for (const c of controls) if (c.refused === undefined) pause = fold(pause, c);
  return pause;
}

/** The pause that stands after control `c`, which is not refused, when `pause` stood before it. */
function fold(pause: Pause | null, c: Control): Pause | null {
  switch (c.action) {
    case "start":
      return null;
    case "compacted":
      return c.pause ?? null;
    case "resume":
      return pause !== null && canLift(c.by, pause.by) ? null : pause;
    case "pause": {
      const owner = pauseOwner(pause?.by ?? null, c.by);
      return owner === c.by ? { by: owner, why: c.payload } : pause;
    }
    default:
      return pause;
  }
}

/** Who lifted the last pause of `controls`, in order, or null while a pause stands or none was lifted. */
export function liftedBy(controls: Control[]): string | null {
  let pause: Pause | null = null;
  let lifter: string | null = null;
  for (const c of controls) {
    if (c.refused !== undefined) continue;
    const before = pause;
    pause = fold(pause, c);
    if (pause !== null) lifter = null;
    else if (before !== null) lifter = c.action === "compacted" ? null : c.by;
  }
  return lifter;
}

/** The controls file of the loop whose folder is `dir`. */
export function controlsPath(dir: string): string {
  return join(dir, CONTROLS_FILE);
}

/** The name of the mutex that guards the controls file of the loop whose folder is `dir`. */
function mutexName(dir: string): string {
  const key = Bun.hash(dir.replaceAll("/", "\\").replace(/\\+$/, "").toLowerCase()).toString(16);
  return `Local\\looper-kit-controls-${key}`;
}

/** Takes the controls mutex of the loop whose folder is `dir`, waiting while another process holds it. Close it to release it. */
async function lock(dir: string): Promise<Mutex> {
  const m = openMutex(mutexName(dir));
  while (!m.tryTake()) await Bun.sleep(LOCK_EVERY_MS);
  return m;
}

/** One control as it is read back, with the byte offset just past its line. */
export interface Read {
  control: Control;
  end: number;
}

/** Whether `v` has the shape of a control. */
function isControl(v: unknown): v is Control {
  const c = v as Control;
  return (
    typeof c === "object" && c !== null && typeof c.at === "number" && typeof c.by === "string" && typeof c.action === "string" && typeof c.payload === "string"
  );
}

/** Reads the controls file of one loop from an offset on. */
export class ControlReader {
  /** Lines that did not parse, or were too long, since the reader was made. */
  skipped = 0;

  constructor(
    readonly dir: string,
    /** The byte offset the next read starts at. */
    public offset = 0,
  ) {}

  /** The file's size now, or 0 when there is none. */
  size(): number {
    const path = controlsPath(this.dir);
    return existsSync(path) ? statSync(path).size : 0;
  }

  /** The complete lines past the offset, at most `READ_BYTES` of them, and moves the offset past them. An offset past the file's end starts again at 0. */
  read(): Read[] {
    const path = controlsPath(this.dir);
    let fd: number;
    try {
      fd = openSync(path, "r");
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code === "ENOENT") return [];
      throw e;
    }
    let buf: Buffer;
    let got: number;
    try {
      const size = fstatSync(fd).size;
      if (this.offset > size) this.offset = 0;
      buf = Buffer.alloc(Math.min(size - this.offset, READ_BYTES));
      got = buf.length === 0 ? 0 : readSync(fd, buf, 0, buf.length, this.offset);
    } finally {
      closeSync(fd);
    }
    const out: Read[] = [];
    let start = 0;
    for (let nl = buf.indexOf(0x0a, start); nl !== -1 && nl < got; nl = buf.indexOf(0x0a, start)) {
      const end = this.offset + nl + 1;
      if (nl + 1 - start > LINE_BYTES) {
        this.skipped++;
      } else {
        try {
          const v = JSON.parse(buf.subarray(start, nl).toString("utf8"));
          if (isControl(v)) out.push({ control: v, end });
          else this.skipped++;
        } catch {
          this.skipped++;
        }
      }
      start = nl + 1;
    }
    // A full read with no newline in it holds a line far past the bound, which is skipped whole.
    if (start === 0 && got === READ_BYTES) {
      this.skipped++;
      start = got;
    }
    this.offset += start;
    return out;
  }
}

/** Every control in the file of the loop whose folder is `dir`, read in pieces of `READ_BYTES`. */
export function readControls(dir: string): Control[] {
  const r = new ControlReader(dir);
  const all: Control[] = [];
  for (let page = r.read(); page.length > 0; page = r.read()) for (const c of page) all.push(c.control);
  return all;
}

/** A control to append: `at` defaults to now and `payload` to empty. */
export interface Ask {
  by: string;
  action: string;
  payload?: string;
  at?: number;
}

/** Writes `line` at the end of the file at `path`, ending a torn last line first. */
function appendLine(path: string, line: string): void {
  const fd = openSync(path, "a+");
  try {
    const size = fstatSync(fd).size;
    let lead = "";
    if (size > 0) {
      const last = Buffer.alloc(1);
      readSync(fd, last, 0, 1, size - 1);
      if (last[0] !== 0x0a) lead = "\n";
    }
    writeSync(fd, `${lead}${line}\n`);
  } finally {
    closeSync(fd);
  }
}

/**
 * Appends `ask` to the controls of the loop whose folder is `dir`, and returns the control written. A
 * `resume` that cannot lift the standing pause is written refused, and throws the reason. An unknown
 * action, or a line past `LINE_BYTES`, throws and writes nothing.
 */
export async function appendControl(dir: string, ask: Ask): Promise<Control> {
  if (!(ACTIONS as readonly string[]).includes(ask.action)) throw new Refused(`there is no control "${ask.action}"`);
  const c: Control = { at: ask.at ?? Date.now(), by: ask.by, action: ask.action, payload: ask.payload ?? "" };
  mkdirSync(dir, { recursive: true });
  const m = await lock(dir);
  try {
    if (c.action === "resume") {
      const p = standingPause(readControls(dir));
      if (p !== null && !canLift(c.by, p.by)) c.refused = `${c.by} cannot lift the pause ${p.by} made, so the loop stays paused`;
    }
    const line = JSON.stringify(c);
    if (Buffer.byteLength(line) + 1 > LINE_BYTES) throw new Refused(`a control takes at most ${LINE_BYTES} bytes`);
    appendLine(controlsPath(dir), line);
  } finally {
    m.close();
  }
  if (c.refused !== undefined) throw new Refused(c.refused);
  return c;
}

/**
 * Compacts the controls file `r` reads once `r` has read past `CONTROLS_BYTES`, and returns the lines it
 * read first, which the caller acts on. Under the mutex it reads the rest, and replaces the file with one
 * `compacted` line that carries the pause standing after all of it. `r`'s offset moves past that line.
 * Returns null when the file is not due.
 */
export async function compact(r: ControlReader): Promise<Read[] | null> {
  if (r.offset <= CONTROLS_BYTES) return null;
  const m = await lock(r.dir);
  try {
    const rest: Read[] = [];
    for (let page = r.read(); page.length > 0; page = r.read()) rest.push(...page);
    if (r.offset < r.size()) return rest;
    const pause = standingPause(readControls(r.dir));
    const line: Control = { at: Date.now(), by: "looper", action: "compacted", payload: "", pause };
    writeState(controlsPath(r.dir), line);
    r.offset = r.size();
    return rest;
  } finally {
    m.close();
  }
}
