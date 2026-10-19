// A session's records as a reader pages them: its events by `seq`, and the events whose summary holds a
// text. It ports the event and search queries of `crates/looper-store/src/query.rs`, read from the
// session's `events/` log (`../store/segments.ts`) where looper reads its store.
//
// - A session is `sessions/<n>/` in the loop's folder, with `session.json` (`SessionRecord`) and the
//   `events/` log that `serve` in `session.ts` writes.
// - A page holds the events from a `seq` on, in order. Its `next` is the `seq` of the page after it when
//   the page was full, and null otherwise, as the API's is.
// - A search matches a summary that holds the text, ignoring case in ASCII letters only, as looper's
//   SQLite `lower` does. It gives the matching events' `seq`s, a page at a time.
//
// # The bounds it keeps
//
// - A page or a search holds at most `PAGE_EVENTS` events.
// - A search reads the log one page of `PAGE_RECORDS` at a time, so its memory is one page whatever the
//   session's length. Its time grows with the events it passes over, at most the log's 64 MiB bound.

import { existsSync } from "node:fs";
import { join } from "node:path";
import { PAGE_RECORDS, readLog } from "../store/segments.ts";
import { readState } from "../store/state.ts";
import { SESSION_FILE, type SessionRecord } from "./driver.ts";

/** The most events one page or one search gives. */
export const PAGE_EVENTS = PAGE_RECORDS;

/** One event as a reader gets it. */
export interface EventView {
  seq: number;
  ts: number;
  kind: string;
  summary: string;
  len: number;
  tool: string | null;
}

export interface EventsPage {
  events: EventView[];
  /** The `seq` of the next page when this one was full, or null. */
  next: number | null;
}

export interface SearchPage {
  hits: number[];
  /** The `from` of the next page when this one was full, or null. */
  next: number | null;
}

/** The folder of session `n` of the loop whose folder is `dir`. */
export function sessionDir(dir: string, n: number): string {
  return join(dir, "sessions", String(n));
}

/** Session `n`'s record, or undefined when the loop has no such session. */
export function sessionRecord(dir: string, n: number): SessionRecord | undefined {
  const at = sessionDir(dir, n);
  if (!existsSync(at)) return undefined;
  return readState<SessionRecord>(join(at, SESSION_FILE));
}

function view(r: Record<string, unknown>): EventView {
  return {
    seq: r.seq as number,
    ts: typeof r.ts === "number" ? r.ts : 0,
    kind: typeof r.kind === "string" ? r.kind : "",
    summary: typeof r.summary === "string" ? r.summary : "",
    len: typeof r.len === "number" ? r.len : 0,
    tool: typeof r.tool === "string" ? r.tool : null,
  };
}

function clamp(limit: number): number {
  return Math.max(1, Math.min(limit, PAGE_EVENTS));
}

/** The events of the session whose folder is `at`, from `seq` `from` on, at most `limit`. */
export function events(at: string, from: number, limit: number = PAGE_EVENTS): EventsPage {
  const want = clamp(limit);
  const page = readLog(join(at, "events"), Math.max(1, from), want);
  const out = page.records.map(view);
  return { events: out, next: out.length === want ? (out.at(-1) as EventView).seq + 1 : null };
}

/** ASCII letters in lower case, and every other character as it is. */
function lower(text: string): string {
  return text.replace(/[A-Z]/g, (c) => c.toLowerCase());
}

/** The `seq` of each event from `from` on whose summary holds `text`, at most `limit`. */
export function search(at: string, text: string, from: number, limit: number = PAGE_EVENTS): SearchPage {
  const want = clamp(limit);
  const needle = lower(text);
  const hits: number[] = [];
  let seq = Math.max(1, from);
  for (;;) {
    const page = readLog(join(at, "events"), seq);
    for (const r of page.records) {
      if (typeof r.summary === "string" && lower(r.summary).includes(needle)) hits.push(r.seq);
      if (hits.length === want) return { hits, next: r.seq + 1 };
    }
    if (page.records.length === 0 || page.next <= seq) return { hits, next: null };
    seq = page.next;
  }
}
