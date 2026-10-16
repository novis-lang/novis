// The progress events a job writes for looper, which shows each item live, keeps its times and
// estimates the time left. The contract is looper's `docs/progress-format.md`; this module writes it.
//
// Each event is one JSON line, `{"v":1,"t":<ms>,"event":<kind>,...fields}`, appended with its newline in
// one write call to the file `LOOPER_EVENTS` names, so lines from many processes never interleave. With
// the variable unset or empty nothing is written, so a command run outside looper behaves exactly as
// before. Nothing goes to stdout or stderr, and a write that fails is dropped: progress never fails the
// command that reports it. A line is kept under looper's 4 KiB bound by cutting its longest free text
// (`why`, `text`, `name`, `log`); one that still does not fit, which only an absurd id makes, is dropped.
//
// What it spends: one `appendFileSync` per event, which is why a caller writes one per item and never
// one per test.

import { appendFileSync } from "node:fs";

/** The most bytes of one line, without its newline: looper skips a longer one. */
export const MAX_LINE = 4096;
/** The fields a line may be cut in, the free text that no reader keys on. */
const CUTTABLE = ["why", "text", "name", "log"] as const;

export type Outcome = "green" | "red" | "skipped" | "cancelled";

export interface AddFields {
  parent?: string;
  name?: string;
  kind?: string;
  kept?: boolean;
  /** Milliseconds since the epoch of the green a kept item rests on. */
  green_at?: number;
}

export interface EndFields {
  result?: Outcome;
  why?: string;
  log?: string;
  artifacts?: string[];
}

/** The event file this process writes to, or "" when it writes none. */
export function eventsFile(): string {
  return process.env.LOOPER_EVENTS ?? "";
}

/** The line for `event` with `fields` at `t`, cut to fit `MAX_LINE`, or null when it cannot fit. */
export function eventLine(event: string, fields: Record<string, unknown>, t: number = Date.now()): string | null {
  const obj: Record<string, unknown> = { v: 1, t: Math.round(t), event };
  for (const [k, v] of Object.entries(fields)) if (v !== undefined) obj[k] = v;
  let line = JSON.stringify(obj);
  for (;;) {
    const over = Buffer.byteLength(line) - MAX_LINE;
    if (over <= 0) return line;
    let longest: string | null = null;
    for (const k of CUTTABLE) {
      const v = obj[k];
      if (typeof v === "string" && v.length > 0 && (longest === null || v.length > (obj[longest] as string).length)) longest = k;
    }
    if (longest === null) return null;
    const v = obj[longest] as string;
    // A character is at least one byte, so cutting `over` of them, and the ellipsis's three, fits or
    // makes the field empty; the loop then cuts the next longest.
    const keep = Math.max(0, v.length - over - 4);
    obj[longest] = keep > 0 ? `${v.slice(0, keep)}…` : "";
    line = JSON.stringify(obj);
  }
}

/** Appends one event, or does nothing when `LOOPER_EVENTS` is unset or empty. Never throws. */
export function emit(event: string, fields: Record<string, unknown> = {}): void {
  const file = eventsFile();
  if (!file) return;
  try {
    const line = eventLine(event, fields);
    if (line !== null) appendFileSync(file, `${line}\n`);
  } catch {
    // Progress never fails the command that reports it.
  }
}

/** One typed call per event of the format. */
export const ev = {
  add: (id: string, f: AddFields = {}) => emit("add", { id, ...f, kept: f.kept ? true : undefined, green_at: f.green_at ? Math.round(f.green_at) : undefined }),
  planned: () => emit("planned"),
  wait: (id: string, on: string) => emit("wait", { id, on }),
  start: (id: string, job?: string) => emit("start", { id, job }),
  tick: (id: string, done: number, total?: number) => emit("tick", { id, done: Math.round(done), total: total === undefined ? undefined : Math.round(total) }),
  note: (text: string, id?: string) => emit("note", { id, text }),
  end: (id: string, f: EndFields = {}) => emit("end", { id, ...f, artifacts: f.artifacts?.length ? f.artifacts : undefined }),
  drop: (id: string) => emit("drop", { id }),
  done: (result?: Outcome) => emit("done", { result }),
};
