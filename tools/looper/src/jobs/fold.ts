// A job's progress: the events it appends to `events.ndjson` in its folder, and the `Fold` that turns
// them into one bounded tree of items. It ports `crates/looper-job/src/events.rs`, and
// `docs/progress-format.md` is the contract.
//
// - An `add` declares an item, and `planned` marks the plan complete for now. An `add` after `planned`
//   changes the plan, and its item is marked `late`.
// - An item's `parent` nests it under an item added before it. A parent added later, or one never
//   added, leaves the item at the top.
// - `wait`, `start` and `end` make the item they name when it is unknown. A `tick` or a `note` never
//   does, so a count inside an item stays a count. A `start` of an ended item is a new attempt.
// - An `end` with no result is green. A `done` sets the job's result, and cancels every item still open.
//   So does `Fold.finish` for a job that ended without `done`, which takes its result from the exit.
// - Unknown fields and unknown events are ignored. A line that is not JSON, or holds a wrong type, is
//   skipped and counted in `Fold.bad`. A missing `t` reads as the newest time seen.
// - Each change bumps the fold's revision and stamps the item with it, so a reader asks for what
//   changed since its last read (`Fold.changedSince`).
//
// # The bounds it keeps
//
// - A fold holds at most `MAX_ITEMS` items. It counts each later item in `Fold.over`.
// - A line holds at most `MAX_LINE` bytes. A longer one is skipped and counted in `Fold.longLines`,
//   and the fold holds at most that many bytes of a line not yet ended.
// - A fold reads at most `MAX_BYTES` of a job's events. Past that it is `truncated` and reads nothing
//   more.
// - An item's ticks bump its revision at most once per `TICK_MS` of event time. The fold keeps the
//   newest tick's counts, and counts the rest in `Fold.coalesced`.
// - An item keeps only its newest note, wait and tick.

import { closeSync, openSync, readSync } from "node:fs";

/** The format's version, which a missing `v` reads as. */
export const VERSION = 1;
/** The most items one fold holds. */
export const MAX_ITEMS = 20_000;
/** The most bytes of one line, without its newline. */
export const MAX_LINE = 4 << 10;
/** The most bytes of one job's events a fold reads. */
export const MAX_BYTES = 16 << 20;
/** The least event time between two ticks of one item that each bump its revision: two per second. */
export const TICK_MS = 500;
/** The bytes `Fold.readFrom` reads at a time. */
const CHUNK = 64 << 10;

/** How an item or a job ended. */
export type Outcome = "green" | "red" | "skipped" | "cancelled";
const OUTCOMES: readonly string[] = ["green", "red", "skipped", "cancelled"];

/** What one line says, without its `v` and `t`. */
export type Event =
  | { event: "add"; id: string; parent?: string; name?: string; kind?: string; kept?: boolean; green_at?: number }
  | { event: "planned" }
  | { event: "wait"; id: string; on: string }
  | { event: "start"; id: string; job?: string }
  | { event: "tick"; id: string; done: number; total?: number }
  | { event: "note"; id?: string; text: string }
  | { event: "end"; id: string; result?: Outcome; why?: string; log?: string; artifacts?: string[] }
  | { event: "drop"; id: string }
  | { event: "done"; result?: Outcome }
  | { event: "unknown" };

/** One line of an event file. */
export type Line = Event & { v?: number; t?: number };

/** Where an item stands. `ended` carries its `result`. */
export type State = "planned" | "kept" | "waiting" | "running" | "ended" | "dropped";

/** One item of a job's progress, with the fields `progress --json` prints. */
export interface Item {
  id: string;
  parent: string | null;
  name: string | null;
  kind: string | null;
  state: State;
  /** How it ended, once `state` is `ended`. */
  result: Outcome | null;
  /** Added after the plan was complete. */
  late: boolean;
  green_at: number | null;
  /** What it waits on, while it waits. */
  wait: string | null;
  /** The ms it spent waiting in this attempt. */
  waited_ms: number;
  started: number | null;
  ended: number | null;
  /** How many times it started. */
  attempts: number;
  done: number | null;
  total: number | null;
  note: string | null;
  why: string | null;
  log: string | null;
  artifacts: string[];
  /** The child job that runs it. */
  job: string | null;
  /** The fold's revision at its last change. */
  rev: number;
}

/** An item and what only the fold reads: its parent's index, and when its wait and its tick came. */
interface Held {
  item: Item;
  up: number | null;
  waitT: number | null;
  tickT: number | null;
}

/** How many items stand in each state, leaving dropped items out. */
export interface Totals {
  items: number;
  planned: number;
  kept: number;
  waiting: number;
  running: number;
  green: number;
  red: number;
  skipped: number;
  cancelled: number;
}

/** Totals of zero. */
export function noTotals(): Totals {
  return { items: 0, planned: 0, kept: 0, waiting: 0, running: 0, green: 0, red: 0, skipped: 0, cancelled: 0 };
}

/** Adds an item standing at `state` with `result` to `totals`. A dropped item adds nothing. */
export function count(totals: Totals, item: Item): void {
  if (item.state === "dropped") return;
  totals.items++;
  if (item.state === "ended") totals[item.result ?? "green"]++;
  else totals[item.state]++;
}

/** Whether `item` has yet to end. */
export function open(item: Item): boolean {
  return item.state === "planned" || item.state === "waiting" || item.state === "running";
}

/** The ms the last attempt of `item` ran, once it ended. */
export function tookMs(item: Item): number | null {
  return item.ended === null || item.started === null ? null : Math.max(0, item.ended - item.started);
}

/** The line for `event` at `t`, without its newline. */
export function lineJson(t: number, event: Event): string {
  return JSON.stringify({ t, ...event });
}

function newItem(id: string, late: boolean): Item {
  return {
    id,
    parent: null,
    name: null,
    kind: null,
    state: "planned",
    result: null,
    late,
    green_at: null,
    wait: null,
    waited_ms: 0,
    started: null,
    ended: null,
    attempts: 0,
    done: null,
    total: null,
    note: null,
    why: null,
    log: null,
    artifacts: [],
    job: null,
    rev: 0,
  };
}

function endItem(h: Held, outcome: Outcome, t: number): void {
  h.item.state = "ended";
  h.item.result = outcome;
  h.item.wait = null;
  h.waitT = null;
  h.item.ended = t;
}

/** Thrown inside `parse` for a line of a wrong shape. */
class Bad extends Error {}

/** The decoder of a line, which refuses bytes that are not UTF-8 as serde_json does. */
const utf8 = new TextDecoder("utf-8", { fatal: true });

/** The line `bytes` holds, or null when it is not an event of the right types. */
export function parse(bytes: Uint8Array): Line | null {
  let doc: unknown;
  try {
    doc = JSON.parse(utf8.decode(bytes));
  } catch {
    return null;
  }
  if (typeof doc !== "object" || doc === null || Array.isArray(doc)) return null;
  const o = doc as Record<string, unknown>;
  const absent = (k: string) => o[k] === undefined || o[k] === null;
  const str = (k: string): string => {
    if (typeof o[k] !== "string") throw new Bad();
    return o[k] as string;
  };
  const optStr = (k: string): string | undefined => (absent(k) ? undefined : str(k));
  const num = (k: string): number => {
    const n = o[k];
    if (typeof n !== "number" || !Number.isInteger(n) || n < 0) throw new Bad();
    return n;
  };
  const optNum = (k: string): number | undefined => (absent(k) ? undefined : num(k));
  const optOutcome = (k: string): Outcome | undefined => {
    const r = optStr(k);
    if (r !== undefined && !OUTCOMES.includes(r)) throw new Bad();
    return r as Outcome | undefined;
  };
  try {
    const head: { v?: number; t?: number } = {};
    const v = optNum("v");
    const t = optNum("t");
    if (v !== undefined) head.v = v;
    if (t !== undefined) head.t = t;
    const event = ((): Event => {
      switch (str("event")) {
        case "add": {
          if (!absent("kept") && typeof o.kept !== "boolean") throw new Bad();
          return {
            event: "add",
            id: str("id"),
            parent: optStr("parent"),
            name: optStr("name"),
            kind: optStr("kind"),
            kept: o.kept === true,
            green_at: optNum("green_at"),
          };
        }
        case "planned":
          return { event: "planned" };
        case "wait":
          return { event: "wait", id: str("id"), on: str("on") };
        case "start":
          return { event: "start", id: str("id"), job: optStr("job") };
        case "tick":
          return { event: "tick", id: str("id"), done: num("done"), total: optNum("total") };
        case "note":
          return { event: "note", id: optStr("id"), text: str("text") };
        case "end": {
          const artifacts = absent("artifacts") ? [] : o.artifacts;
          if (!Array.isArray(artifacts) || artifacts.some((a) => typeof a !== "string")) throw new Bad();
          return { event: "end", id: str("id"), result: optOutcome("result"), why: optStr("why"), log: optStr("log"), artifacts };
        }
        case "drop":
          return { event: "drop", id: str("id") };
        case "done":
          return { event: "done", result: optOutcome("result") };
        default:
          return { event: "unknown" };
      }
    })();
    return { ...head, ...event };
  } catch (e) {
    if (e instanceof Bad) return null;
    throw e;
  }
}

/** A job's events folded into one bounded tree of items. */
export class Fold {
  private held: Held[] = [];
  private index = new Map<string, number>();
  private revision = 0;
  /** The bytes of the line not yet ended, or null while the fold skips a line past `MAX_LINE`. */
  private partial: Buffer[] | null = [];
  private partialLen = 0;
  /** Whether a `planned` came. */
  planned = false;
  /** The job's result, once a `done` came or `finish` ran. */
  result: Outcome | null = null;
  /** Whether a `done` came. */
  done = false;
  /** The newest note about the job as a whole. */
  note: string | null = null;
  firstT: number | null = null;
  lastT: number | null = null;
  /** The bytes read, which is where the next read starts. */
  bytes = 0;
  /** Whether `MAX_BYTES` was reached. */
  truncated = false;
  /** Items not held because `MAX_ITEMS` was reached. */
  over = 0;
  /** Lines past `MAX_LINE`. */
  longLines = 0;
  /** Lines that were not an event. */
  bad = 0;
  /** Ticks that bumped no revision. */
  coalesced = 0;

  /** Folds the whole event file at `path`. A missing file is an empty fold. */
  static read(path: string): Fold {
    const fold = new Fold();
    fold.readFrom(path);
    return fold;
  }

  /** Folds what was appended to `path` since the last read. */
  readFrom(path: string): void {
    if (this.truncated) return;
    let fd: number;
    try {
      fd = openSync(path, "r");
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code === "ENOENT") return;
      throw e;
    }
    try {
      const buf = Buffer.alloc(CHUNK);
      while (!this.truncated) {
        const n = readSync(fd, buf, 0, CHUNK, this.bytes);
        if (n === 0) break;
        this.feed(buf.subarray(0, n));
      }
    } finally {
      closeSync(fd);
    }
  }

  /** Folds `bytes`, the next bytes of the file. A line not yet ended waits for the rest. */
  feed(bytes: Uint8Array): void {
    if (this.truncated) return;
    const room = MAX_BYTES - this.bytes;
    let rest = Buffer.from(bytes.buffer, bytes.byteOffset, Math.min(bytes.length, room));
    this.bytes += rest.length;
    if (bytes.length > room) this.truncated = true;
    while (rest.length > 0) {
      const i = rest.indexOf(0x0a);
      if (i < 0) {
        this.piece(rest, false);
        break;
      }
      this.piece(rest.subarray(0, i), true);
      rest = rest.subarray(i + 1);
    }
  }

  private piece(piece: Buffer, ends: boolean): void {
    if (this.partial === null) {
      if (ends) this.resetPartial();
      return;
    }
    if (this.partialLen + piece.length > MAX_LINE) {
      this.longLines++;
      this.resetPartial();
      if (!ends) this.partial = null;
    } else if (!ends) {
      // A read's buffer is reused, so a piece held past this call is copied.
      this.partial.push(Buffer.from(piece));
      this.partialLen += piece.length;
    } else if (this.partialLen === 0) {
      this.line(piece);
    } else {
      const whole = Buffer.concat([...this.partial, piece]);
      this.resetPartial();
      this.line(whole);
    }
  }

  private resetPartial(): void {
    this.partial = [];
    this.partialLen = 0;
  }

  private line(bytes: Buffer): void {
    if (bytes.every((b) => b === 0x20 || b === 0x09 || b === 0x0d || b === 0x0c)) return;
    const line = parse(bytes);
    if (line === null) this.bad++;
    else this.apply(line);
  }

  /** Folds one event. */
  apply(line: Line): void {
    const t = line.t ?? this.lastT ?? 0;
    this.firstT ??= t;
    this.lastT = Math.max(this.lastT ?? t, t);
    switch (line.event) {
      case "add": {
        const i = this.make(line.id);
        if (i === null) return;
        const h = this.held[i] as Held;
        if (line.parent !== undefined) {
          const up = this.index.get(line.parent);
          h.item.parent = line.parent;
          h.up = up !== undefined && up < i ? up : null;
        }
        if (line.name !== undefined) h.item.name = line.name;
        if (line.kind !== undefined) h.item.kind = line.kind;
        if (line.kept) {
          h.item.state = "kept";
          h.item.green_at = line.green_at ?? null;
        } else if (h.item.state === "dropped") h.item.state = "planned";
        this.bump(i);
        return;
      }
      case "planned":
        this.planned = true;
        this.revision++;
        return;
      case "wait": {
        const i = this.make(line.id);
        if (i === null) return;
        const h = this.held[i] as Held;
        h.item.state = "waiting";
        h.item.wait = line.on;
        h.waitT = t;
        this.bump(i);
        return;
      }
      case "start": {
        const i = this.make(line.id);
        if (i === null) return;
        const h = this.held[i] as Held;
        const it = h.item;
        it.waited_ms = h.waitT === null ? 0 : Math.max(0, t - h.waitT);
        h.waitT = null;
        it.state = "running";
        it.result = null;
        it.wait = null;
        it.started = t;
        it.ended = null;
        it.attempts++;
        it.done = null;
        it.total = null;
        h.tickT = null;
        it.why = null;
        it.log = null;
        it.artifacts = [];
        it.job = line.job ?? null;
        this.bump(i);
        return;
      }
      case "tick": {
        const i = this.index.get(line.id);
        if (i === undefined) return;
        const h = this.held[i] as Held;
        h.item.done = line.done;
        if (line.total !== undefined) h.item.total = line.total;
        if (h.tickT !== null && t < h.tickT + TICK_MS) this.coalesced++;
        else {
          h.tickT = t;
          this.bump(i);
        }
        return;
      }
      case "note": {
        if (line.id === undefined) {
          this.note = line.text;
          this.revision++;
          return;
        }
        const i = this.index.get(line.id);
        if (i === undefined) return;
        (this.held[i] as Held).item.note = line.text;
        this.bump(i);
        return;
      }
      case "end": {
        const i = this.make(line.id);
        if (i === null) return;
        const h = this.held[i] as Held;
        endItem(h, line.result ?? "green", t);
        h.item.why = line.why ?? null;
        h.item.log = line.log ?? null;
        h.item.artifacts = line.artifacts ?? [];
        this.bump(i);
        return;
      }
      case "drop": {
        const i = this.index.get(line.id);
        if (i === undefined) return;
        (this.held[i] as Held).item.state = "dropped";
        this.bump(i);
        return;
      }
      case "done": {
        const red = this.held.some((h) => h.item.state === "ended" && h.item.result === "red");
        this.done = true;
        this.result = line.result ?? (red ? "red" : "green");
        this.cancelOpen(t);
        return;
      }
      case "unknown":
        return;
    }
  }

  /**
   * Closes the fold of a job that ended with `exit`, or was killed with none. A torn last line is
   * skipped. Without a `done`, every open item is cancelled, and the exit gives the job's result.
   */
  finish(exit: number | null, t: number): void {
    this.resetPartial();
    if (this.done) return;
    this.result = exit === null ? "cancelled" : exit === 0 ? "green" : "red";
    this.cancelOpen(Math.max(t, this.lastT ?? 0));
  }

  private cancelOpen(t: number): void {
    this.revision++;
    for (const h of this.held) {
      if (!open(h.item)) continue;
      endItem(h, "cancelled", t);
      h.item.rev = this.revision;
    }
  }

  /** The index of the item `id`, made when it is unknown and the fold has room. */
  private make(id: string): number | null {
    const known = this.index.get(id);
    if (known !== undefined) return known;
    if (this.held.length >= MAX_ITEMS) {
      this.over++;
      return null;
    }
    const i = this.held.length;
    this.index.set(id, i);
    this.held.push({ item: newItem(id, this.planned), up: null, waitT: null, tickT: null });
    return i;
  }

  private bump(i: number): void {
    this.revision++;
    (this.held[i] as Held).item.rev = this.revision;
  }

  /** The fold's revision, which every change raises. */
  rev(): number {
    return this.revision;
  }

  /** Every item, dropped ones too, in the order each was first named. */
  items(): Item[] {
    return this.held.map((h) => h.item);
  }

  /** The item `id`. */
  get(id: string): Item | undefined {
    const i = this.index.get(id);
    return i === undefined ? undefined : (this.held[i] as Held).item;
  }

  /** The items changed after revision `rev`, dropped ones too. */
  changedSince(rev: number): Item[] {
    return this.items().filter((item) => item.rev > rev);
  }

  /**
   * The tree, depth first: each item with its depth, children in the order each was first named. A
   * dropped item and everything under it are left out.
   */
  tree(): [number, Item][] {
    const kids: number[][] = this.held.map(() => []);
    const roots: number[] = [];
    this.held.forEach((h, i) => (h.up === null ? roots.push(i) : (kids[h.up] as number[]).push(i)));
    const out: [number, Item][] = [];
    const stack: [number, number][] = roots.toReversed().map((i) => [0, i]);
    for (let next = stack.pop(); next !== undefined; next = stack.pop()) {
      const [depth, i] = next;
      const item = (this.held[i] as Held).item;
      if (item.state === "dropped") continue;
      out.push([depth, item]);
      for (const k of (kids[i] as number[]).toReversed()) stack.push([depth + 1, k]);
    }
    return out;
  }

  /** How many items stand in each state. */
  totals(): Totals {
    const totals = noTotals();
    for (const [, item] of this.tree()) count(totals, item);
    return totals;
  }
}
