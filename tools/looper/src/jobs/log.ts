// A job's log: its first `LOG_PART` bytes in `log`, and its newest output rotating through `tail` and
// `tail.1`, as `crates/looper-job/src/log.rs` writes it.
//
// - `LogWriter` fills `log`, then `tail`. When `tail` is full it becomes `tail.1`, and the `tail.1`
//   before it is dropped. Each rotation rewrites `dropped`, the count of bytes dropped so far, through
//   tmp+rename. A log is at most three parts.
// - `readLines` joins the parts back into one text and hands over the lines it is asked for. Where
//   bytes were dropped it puts one line saying how many, so a reader sees the head, the gap and the
//   newest output.
//
// # The bounds it keeps
//
// - A log on disk is at most three parts of the writer's part size.
// - `readLines` reads `READ_CHUNK` bytes at a time and holds one line, or with `last` that many lines.

import { closeSync, existsSync, fstatSync, openSync, readFileSync, readSync, renameSync, statSync, writeSync } from "node:fs";
import { join } from "node:path";
import { writeState } from "../store/state.ts";

/** The size of each of a log's three parts: 8 MiB. */
export const LOG_PART = 8 << 20;

/** The parts of a log, in the order a read joins them, and the file that counts what was dropped. */
export const HEAD = "log";
export const TAIL = "tail";
export const TAIL_OLD = "tail.1";
export const DROPPED = "dropped";

/** The bytes a read takes from a part at a time. */
const READ_CHUNK = 64 << 10;

/** Appends a job's output to its log, as it arrives. */
export class LogWriter {
  private readonly part: number;
  private head: number;
  private headLen: number;
  private tail: number | null = null;
  private tailLen = 0;
  private dropped = 0;

  /** The writer of the log in `dir`, with parts of `part` bytes. */
  constructor(
    private readonly dir: string,
    part = LOG_PART,
  ) {
    this.part = Math.max(1, part);
    this.head = openSync(join(dir, HEAD), "a");
    this.headLen = fstatSync(this.head).size;
  }

  /** Appends `buf`, rotating the tail as it fills. */
  write(buf: Uint8Array): void {
    let at = 0;
    while (at < buf.length) {
      let n: number;
      if (this.headLen < this.part) {
        n = Math.min(buf.length - at, this.part - this.headLen);
        writeAll(this.head, buf.subarray(at, at + n));
        this.headLen += n;
      } else {
        if (this.tail === null || this.tailLen >= this.part) this.rotate();
        n = Math.min(buf.length - at, this.part - this.tailLen);
        writeAll(this.tail as number, buf.subarray(at, at + n));
        this.tailLen += n;
      }
      at += n;
    }
  }

  /** Opens the first tail, or moves a full one to `tail.1` and opens a fresh one. */
  private rotate(): void {
    if (this.tail !== null) {
      closeSync(this.tail);
      this.tail = null;
      const old = join(this.dir, TAIL_OLD);
      if (existsSync(old)) this.dropped += statSync(old).size;
      renameSync(join(this.dir, TAIL), old);
      if (this.dropped > 0) writeState(join(this.dir, DROPPED), this.dropped);
    }
    this.tail = openSync(join(this.dir, TAIL), "w");
    this.tailLen = 0;
  }

  /** Closes the log's files. */
  close(): void {
    closeSync(this.head);
    if (this.tail !== null) closeSync(this.tail);
    this.tail = null;
  }
}

function writeAll(fd: number, buf: Uint8Array): void {
  let at = 0;
  while (at < buf.length) at += writeSync(fd, buf, at, buf.length - at);
}

/** The bytes of a file, 0 when it is missing. */
function sizeOf(path: string): number {
  try {
    return statSync(path).size;
  } catch {
    return 0;
  }
}

/** The bytes of all three parts of the log in `dir`, on disk now. */
export function size(dir: string): number {
  return [HEAD, TAIL_OLD, TAIL].reduce((sum, p) => sum + sizeOf(join(dir, p)), 0);
}

/** The bytes of the log in `dir` that rotation dropped. */
export function droppedBytes(dir: string): number {
  try {
    return Number(JSON.parse(readFileSync(join(dir, DROPPED), "utf8"))) || 0;
  } catch {
    return 0;
  }
}

/** Which lines of a log `readLines` hands over. */
export type Lines =
  | { all: true }
  /** `count` lines from the 1-based line `from`, or every line from it without a count. */
  | { from: number; count?: number }
  /** The last `last` lines. */
  | { last: number };

/** Hands each byte chunk of the log in `dir` to `out` in order: the head, the gap line, then the tails. */
function eachChunk(dir: string, out: (chunk: Buffer) => boolean): void {
  const dropped = droppedBytes(dir);
  const parts: (string | Buffer)[] = [HEAD];
  if (dropped > 0) {
    const lead = endsInNewline(join(dir, HEAD)) ? "" : "\n";
    parts.push(Buffer.from(`${lead}[looper: ${dropped} bytes of output were dropped here]\n`));
  }
  parts.push(TAIL_OLD, TAIL);
  for (const part of parts) {
    if (typeof part !== "string") {
      if (!out(part)) return;
      continue;
    }
    let fd: number;
    try {
      fd = openSync(join(dir, part), "r");
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code === "ENOENT") continue;
      throw e;
    }
    try {
      const buf = Buffer.alloc(READ_CHUNK);
      for (;;) {
        const n = readSync(fd, buf, 0, buf.length, null);
        if (n === 0) break;
        if (!out(Buffer.from(buf.subarray(0, n)))) return;
      }
    } finally {
      closeSync(fd);
    }
  }
}

function endsInNewline(path: string): boolean {
  let fd: number;
  try {
    fd = openSync(path, "r");
  } catch {
    return true;
  }
  try {
    const len = fstatSync(fd).size;
    if (len === 0) return true;
    const last = Buffer.alloc(1);
    readSync(fd, last, 0, 1, len - 1);
    return last[0] === 0x0a;
  } finally {
    closeSync(fd);
  }
}

/** Hands each chosen line of the log in `dir` to `out`, with its newline when it has one. */
export function readLines(dir: string, pick: Lines, out: (line: Buffer) => void): void {
  const last: Buffer[] = [];
  let n = 0;
  let pending: Buffer[] = [];
  /** Takes one whole line. Says whether the read goes on. */
  const take = (line: Buffer): boolean => {
    n++;
    if ("all" in pick) out(line);
    else if ("from" in pick) {
      if (n < pick.from) return true;
      if (pick.count !== undefined && n >= pick.from + pick.count) return false;
      out(line);
    } else if (pick.last > 0) {
      if (last.length === pick.last) last.shift();
      last.push(line);
    }
    return true;
  };
  let going = true;
  eachChunk(dir, (chunk) => {
    let at = 0;
    for (;;) {
      const nl = chunk.indexOf(0x0a, at);
      if (nl < 0) break;
      const line = Buffer.concat([...pending, chunk.subarray(at, nl + 1)]);
      pending = [];
      at = nl + 1;
      if (!take(line)) {
        going = false;
        return false;
      }
    }
    if (at < chunk.length) pending.push(chunk.subarray(at));
    return true;
  });
  if (going && pending.length > 0) take(Buffer.concat(pending));
  for (const line of last) out(line);
}
