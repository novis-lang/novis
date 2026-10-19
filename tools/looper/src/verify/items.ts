// The verify run's own progress items, which the run appends to its job's event file in looper's
// progress format (`docs/progress-format.md`). It ports `crates/looper/src/cmd/verify/items.rs`.
//
// - Each step is an item, `step:<name>`, added as the run starts. A step that is one unit is that
//   unit's item. The command it runs is the child job its `start` names, so the command's own items
//   nest under it.
// - A `cargo test` step adds an item per test binary once its build has listed them, each under the
//   step's item and named by its unit id. The plan is complete once the binaries are listed, or as the
//   run starts when no step is `cargo test`.
// - A step or a binary whose last green still holds is added `kept`, with the time that green was
//   recorded as its `green_at`.
// - A wait for a heavy-job slot is a `wait` on `slot`. A unit killed for new content ends `cancelled`,
//   and its next run is a new attempt of the same id.
// - The run ends with `done`. A run that dies writes none, so a reader's fold cancels its open items.
//
// # The bounds it keeps
//
// It opens the file once per line and holds nothing between lines. It writes one line per item event
// and none per test, and a `why` past `WHY_CHARS` is cut.

import { closeSync, openSync, writeSync } from "node:fs";
import type { Step } from "../goal/config.ts";
import { type Event, lineJson, MAX_LINE, type Outcome } from "../jobs/fold.ts";
import type { Store } from "./select.ts";

/** The most characters of a `why` an `end` carries. */
export const WHY_CHARS = 300;

/** The kind of a step's item. */
export const STEP = "step";
/** The kind of a test binary's item. */
export const BINARY = "test-binary";

/** Where the run's items go: an event file, or nowhere without one. */
export class Items {
  private readonly path: string | null;
  private isPlanned = false;

  /** Appends to the event file at `path`, or writes nothing without one. */
  constructor(path: string | null) {
    this.path = path;
  }

  /** Appends `event` as one line with one write. A line past `MAX_LINE`, or a file it cannot write, is left out. */
  private write(event: Event): void {
    if (this.path === null) return;
    const line = lineJson(Date.now(), event);
    if (Buffer.byteLength(line) > MAX_LINE) return;
    try {
      const fd = openSync(this.path, "a");
      try {
        writeSync(fd, `${line}\n`);
      } finally {
        closeSync(fd);
      }
    } catch {}
  }

  /** Adds the item `id` of `kind` under `parent`, named `name`. */
  add(id: string, parent: string | null, name: string, kind: string): void {
    this.write({ event: "add", id, ...(parent === null ? {} : { parent }), name, kind });
  }

  /** Adds the item `id` as kept, resting on the green the store recorded last for it. */
  kept(store: Store, id: string, parent: string | null, name: string, kind: string): void {
    const at = greenAt(store, id);
    this.write({ event: "add", id, ...(parent === null ? {} : { parent }), name, kind, kept: true, ...(at === null ? {} : { green_at: at }) });
  }

  /** Adds an item per step of `steps`, and completes the plan when none is `cargo test`. */
  steps(steps: readonly Step[], isCargo: (argv: readonly string[]) => boolean): void {
    for (const step of steps) this.add(`step:${step.name}`, null, step.name, STEP);
    if (!steps.some((s) => isCargo(s.run))) this.planned();
  }

  /** Marks the plan complete, once. */
  planned(): void {
    if (this.isPlanned) return;
    this.isPlanned = true;
    this.write({ event: "planned" });
  }

  /** Says that `id` waits for a heavy-job slot. */
  waits(id: string): void {
    this.write({ event: "wait", id, on: "slot" });
  }

  /** Starts an attempt of `id`, run by the child job `job`. */
  start(id: string, job: string | null): void {
    this.write({ event: "start", id, ...(job === null ? {} : { job }) });
  }

  /** Ends `id` with `outcome`, `why` and its `log`. */
  end(id: string, outcome: Outcome, why: string, log: string | null): void {
    const cut = [...why.trim()].slice(0, WHY_CHARS).join("");
    this.write({ event: "end", id, result: outcome, ...(cut === "" ? {} : { why: cut }), ...(log === null ? {} : { log }) });
  }

  /** Ends the run, green or red. */
  done(green: boolean): void {
    this.write({ event: "done", result: outcome(green) });
  }
}

/** When the store recorded the last run of the unit `id`, while that run was green. For a kept unit it is its newest green. */
function greenAt(store: Store, id: string): number | null {
  const row = store.unit(id);
  return row === undefined || row.red ? null : row.at;
}

/** `green` as an item's outcome. */
export function outcome(green: boolean): Outcome {
  return green ? "green" : "red";
}
