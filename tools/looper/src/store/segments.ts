// A log in a project's data folder: one directory of NDJSON segments, each record one line with a `seq`.
// `docs/kit-contract.md` § *The data folder* is the contract a reader of the files relies on.
//
// - A segment is named by the `seq` of its first record, as `<seq padded to 16 digits>.ndjson`. So the
//   names sort in order, and a reader finds the segment holding a `seq` from the names alone.
// - `seq` starts at 1 and rises by one per record. A record's own `seq` field is replaced by the log's.
// - A line is written with its newline in one write call. Only lines that end in a newline count. A
//   torn last line, left by a writer killed mid-write, is skipped. A writer that opens the log after
//   one ends it with a newline, so the next record starts on a line of its own.
// - A line that does not parse is skipped and counted, never fatal.
// - The log is not synced to disk per record. A power loss costs the newest records, never older ones.
// - One `LogWriter` writes a log at a time. A later goal that needs two writers takes a lock.
//
// The bounds:
// - `SEGMENT_BYTES`, 1 MiB: a segment rolls before a line would take it past this size.
// - `MAX_SEGMENTS`, 64: the oldest segments are deleted past this count, so a log holds at most 64 MiB.
// - `LINE_BYTES`, 64 KiB: a longer record is refused on append, and a longer line is skipped on read.
// - `PAGE_RECORDS`, 1000: the most records one `readLog` call returns.

import { closeSync, mkdirSync, openSync, readdirSync, readFileSync, statSync, unlinkSync, writeSync } from "node:fs";
import { join } from "node:path";

export const SEGMENT_BYTES = 1024 * 1024;
export const MAX_SEGMENTS = 64;
export const LINE_BYTES = 64 * 1024;
export const PAGE_RECORDS = 1000;

/** A record as it is read back: its fields and the `seq` the log gave it. */
export type LogRecord = { seq: number } & Record<string, unknown>;

/** The bounds of one log. A test passes small ones; every other caller takes the defaults. */
export interface Bounds {
  segmentBytes: number;
  maxSegments: number;
}

const defaults: Bounds = { segmentBytes: SEGMENT_BYTES, maxSegments: MAX_SEGMENTS };
const suffix = ".ndjson";
const newline = 0x0a;

/** The file name of the segment whose first record is `seq`. */
export function segmentName(seq: number): string {
  return `${String(seq).padStart(16, "0")}${suffix}`;
}

/** The first `seq` of each segment in `dir`, oldest first. A missing directory has none. */
export function segments(dir: string): number[] {
  let names: string[];
  try {
    names = readdirSync(dir);
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") return [];
    throw e;
  }
  return names
    .filter((n) => /^\d{16}\.ndjson$/.test(n))
    .map((n) => Number(n.slice(0, 16)))
    .sort((a, b) => a - b);
}

/** The whole lines of one segment's bytes, each parsed, and the count of lines skipped. */
function parse(bytes: Uint8Array): { records: LogRecord[]; skipped: number } {
  const records: LogRecord[] = [];
  let skipped = 0;
  const decoder = new TextDecoder();
  let start = 0;
  for (let end = bytes.indexOf(newline); end >= 0; end = bytes.indexOf(newline, start)) {
    const line = bytes.subarray(start, end);
    start = end + 1;
    if (line.length === 0) continue;
    if (line.length + 1 > LINE_BYTES) {
      skipped++;
      continue;
    }
    try {
      const value: unknown = JSON.parse(decoder.decode(line));
      if (typeof value === "object" && value !== null && typeof (value as LogRecord).seq === "number") {
        records.push(value as LogRecord);
      } else skipped++;
    } catch {
      skipped++;
    }
  }
  return { records, skipped };
}

/** Appends records to the log in `dir`, rolling and deleting segments to keep its bounds. */
export class LogWriter {
  private fd: number;
  private size: number;
  private next: number;
  private first: number[];

  constructor(
    readonly dir: string,
    private readonly bounds: Bounds = defaults,
  ) {
    mkdirSync(dir, { recursive: true });
    this.first = segments(dir);
    const newest = this.first.at(-1);
    if (newest === undefined) {
      this.next = 1;
      this.first = [1];
      this.fd = openSync(join(dir, segmentName(1)), "a");
      this.size = 0;
      return;
    }
    const path = join(dir, segmentName(newest));
    const bytes = readFileSync(path);
    this.next = (parse(bytes).records.at(-1)?.seq ?? newest - 1) + 1;
    this.fd = openSync(path, "a");
    this.size = bytes.length;
    if (bytes.length > 0 && bytes[bytes.length - 1] !== newline) {
      writeSync(this.fd, "\n");
      this.size++;
    }
  }

  /** The `seq` the next record gets. */
  get nextSeq(): number {
    return this.next;
  }

  /** Appends one record and gives its `seq`. A record whose line is over `LINE_BYTES` is refused. */
  append(record: Record<string, unknown>): number {
    const { seq: _, ...fields } = record;
    const seq = this.next;
    const line = Buffer.from(`${JSON.stringify({ seq, ...fields })}\n`);
    if (line.length > LINE_BYTES) {
      throw new Error(`a record of ${line.length} bytes is over the ${LINE_BYTES}-byte line bound`);
    }
    if (this.size > 0 && this.size + line.length > this.bounds.segmentBytes) this.roll(seq);
    writeSync(this.fd, line);
    this.size += line.length;
    this.next = seq + 1;
    return seq;
  }

  /** Closes the newest segment. The writer takes no record after this. */
  close(): void {
    closeSync(this.fd);
  }

  /** Starts the segment whose first record is `seq`, and deletes the oldest past `maxSegments`. */
  private roll(seq: number): void {
    closeSync(this.fd);
    this.fd = openSync(join(this.dir, segmentName(seq)), "a");
    this.size = 0;
    this.first.push(seq);
    while (this.first.length > this.bounds.maxSegments) {
      const oldest = this.first.shift() as number;
      unlinkSync(join(this.dir, segmentName(oldest)));
    }
  }
}

/** One page of a log. */
export interface Page {
  records: LogRecord[];
  /** The `seq` to ask for next. It equals the asked `seq` when nothing newer is there yet. */
  next: number;
  /** The oldest `seq` the log still holds. A reader that asked for an older one lost the gap. */
  oldest: number;
  /** Lines skipped because they did not parse or were over `LINE_BYTES`. */
  skipped: number;
}

/**
 * Reads up to `limit` records from `seq` on. Only the segments that hold them are read, each whole, so
 * one call reads at most `limit` records and the segments they span.
 */
export function readLog(dir: string, seq: number, limit: number = PAGE_RECORDS): Page {
  const want = Math.max(1, Math.min(limit, PAGE_RECORDS));
  const first = segments(dir);
  const oldest = first[0] ?? 1;
  let at = 0;
  for (let i = 0; i < first.length; i++) if ((first[i] as number) <= seq) at = i;

  const records: LogRecord[] = [];
  let skipped = 0;
  for (let i = at; i < first.length && records.length < want; i++) {
    let bytes: Uint8Array;
    try {
      bytes = readFileSync(join(dir, segmentName(first[i] as number)));
    } catch (e) {
      // The writer deleted the segment after the listing: its records are gone, so read on.
      if ((e as NodeJS.ErrnoException).code === "ENOENT") continue;
      throw e;
    }
    const page = parse(bytes);
    skipped += page.skipped;
    for (const r of page.records) {
      if (r.seq < seq) continue;
      records.push(r);
      if (records.length === want) break;
    }
  }
  const last = records.at(-1);
  return { records, next: last === undefined ? Math.max(seq, oldest) : last.seq + 1, oldest, skipped };
}

/** The total bytes of the log's segments. */
export function logBytes(dir: string): number {
  return segments(dir).reduce((sum, s) => sum + statSync(join(dir, segmentName(s))).size, 0);
}
