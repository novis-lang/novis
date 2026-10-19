// The writer of a job's progress events: one line appended to the file `LOOPER_EVENTS` names, as
// `docs/progress-format.md` § *The file* says. The `event` command (`../cli/event.ts`) and any kit tool
// that reports its progress write through it, and `fold.ts` reads what it wrote.
//
// - It opens the file for append and writes the line and its newline with one write call, so lines from
//   many processes never interleave.
// - With `LOOPER_EVENTS` unset or empty it writes nothing, so a tool runs the same outside a job.
// - A line past `MAX_LINE` is refused, because a fold would skip it.

import { closeSync, openSync, writeSync } from "node:fs";
import { type Event, lineJson, MAX_LINE } from "./fold.ts";
import { EVENTS_VAR } from "./store.ts";

/** Why an event was not written. */
export class EventRefused extends Error {}

/**
 * Appends `event` at `t` to the event file `env` names. Says whether it wrote, which it does not
 * without `LOOPER_EVENTS`. Throws `EventRefused` for a line past `MAX_LINE` or a file it cannot write.
 */
export function appendEvent(event: Event, env: Record<string, string | undefined> = process.env, t = Date.now()): boolean {
  const path = env[EVENTS_VAR];
  if (path === undefined || path === "") return false;
  const line = Buffer.from(`${lineJson(t, event)}\n`);
  if (line.length - 1 > MAX_LINE) {
    throw new EventRefused(`the event is ${line.length - 1} bytes, past the ${MAX_LINE} a line holds`);
  }
  let fd: number;
  try {
    fd = openSync(path, "a");
  } catch (e) {
    throw new EventRefused(`cannot append to ${path}: ${(e as Error).message}`);
  }
  try {
    const wrote = writeSync(fd, line);
    if (wrote !== line.length) throw new EventRefused(`cannot append to ${path}: ${wrote} of ${line.length} bytes were written`);
  } catch (e) {
    if (e instanceof EventRefused) throw e;
    throw new EventRefused(`cannot append to ${path}: ${(e as Error).message}`);
  } finally {
    closeSync(fd);
  }
  return true;
}
