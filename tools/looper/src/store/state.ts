// A state file in a project's data folder: one JSON document, replaced whole. `docs/kit-contract.md`
// § *The data folder* is the contract a reader of the files relies on.
//
// - `writeState` writes the document to `<path>.tmp`, syncs it to disk, and renames it over `path`. So a
//   reader finds the old document or the new one, never a mix, and a crash leaves one of the two.
// - One writer writes a state file at a time, so the one `.tmp` name never clashes. A `.tmp` left by a
//   crashed writer is overwritten by the next write, so a state file leaves at most one behind.
// - Windows refuses a rename, or a read, while another process has the file open without sharing its
//   delete. Both retry up to `RETRIES` times, `RETRY_MS` apart, and then throw.

import { closeSync, fsyncSync, mkdirSync, openSync, readFileSync, renameSync, writeSync } from "node:fs";
import { dirname } from "node:path";

export const RETRIES = 50;
export const RETRY_MS = 10;

const busy = new Set(["EPERM", "EACCES", "EBUSY"]);

/** Runs `op`, again while Windows reports the file busy. */
function retried<T>(op: () => T): T {
  for (let attempt = 1; ; attempt++) {
    try {
      return op();
    } catch (e) {
      if (attempt >= RETRIES || !busy.has((e as NodeJS.ErrnoException).code ?? "")) throw e;
      Bun.sleepSync(RETRY_MS);
    }
  }
}

/** Replaces the state file at `path` with `value` as JSON. */
export function writeState(path: string, value: unknown): void {
  mkdirSync(dirname(path), { recursive: true });
  const tmp = `${path}.tmp`;
  const fd = openSync(tmp, "w");
  try {
    writeSync(fd, `${JSON.stringify(value)}\n`);
    fsyncSync(fd);
  } finally {
    closeSync(fd);
  }
  retried(() => renameSync(tmp, path));
}

/** The document in the state file at `path`, or `undefined` when there is none. */
export function readState<T = unknown>(path: string): T | undefined {
  let text: string;
  try {
    text = retried(() => readFileSync(path, "utf8"));
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") return undefined;
    throw e;
  }
  return JSON.parse(text) as T;
}
