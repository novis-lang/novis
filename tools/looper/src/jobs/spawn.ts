// The spawn of a recorded job: the one place the kit runs a command as a job of the project's store
// (`store.ts`), with its log, its exit and its Job Object.
//
// - `startJob` makes the job's folder, resolves the argv (`program.ts`) and starts it as a tree in a
//   Job Object named after the job's id. The command runs with `LOOPER_JOB` set to the job's id and
//   `LOOPER_EVENTS` naming `events.ndjson` in the job's folder. Its record is written once the
//   command runs, and names the loop that the command's own `LOOPER_LOOP` names. A command that cannot start leaves no folder behind. A job started with `hide` opens
//   its first window without the focus, and a window watcher keeps every window of the tree off the
//   screen until the job ends (`../os/windows-desktop.ts`).
// - This process is the job's runner. It drains the tree's pipe every `PUMP_MS` into the job's log,
//   and hands each chunk to `onOutput` as well. A job with `onError` gets its stderr on a pipe of its
//   own, whose chunks go to the log and to `onError`. A job with `stdin` gets it on a pipe, which the
//   runner fills as the pipe takes it on each drain, then closes.
// - Once the command's own process exits, the runner drains what is left, writes `exit`, and closes the
//   tree, which kills every process the command left running. With `lingerMs` it first waits that long
//   for the output to close, and `held` says when it did not.
// - A job with a project then folds its events with its exit, and records each item that ended green
//   or red in the project's item history (`history.ts`). A history it cannot write costs the runs,
//   never the job.
// - A runner that dies first takes the tree with it, because the job kills on close. The job then
//   reads as lost.
// - Each start first runs the store's sweep when one is due (`Store.sweepDue`).
//
// # The bounds it keeps
//
// - The runner holds one chunk of each pipe at a time, at most `READ_BYTES`. After the command's own
//   process exits it drains at most `DRAIN_READS` chunks, or for at most `lingerMs`.
// - The log keeps its own bound of three parts (`log.ts`).
// - The runner holds one fold once the job ended, within the bounds of `fold.ts`.

import { rmSync } from "node:fs";
import { join } from "node:path";
import { READ_BYTES, spawnTree, type Tree, type WindowWatcher } from "../os/index.ts";
import { LOG_PART, LogWriter } from "./log.ts";
import { Fold } from "./fold.ts";
import { record as recordRuns, runsOf } from "./history.ts";
import { program } from "./program.ts";
import { EVENTS, EVENTS_VAR, EXIT, type JobExit, type JobRecord, jobName, type Kind, LOOP_VAR, PARENT_VAR, RECORD, type Store } from "./store.ts";
import { writeState } from "../store/state.ts";

/** How often the runner drains the tree's pipe while it holds nothing. */
export const PUMP_MS = 20;

/** The most reads the runner drains once the command's own process has exited. */
const DRAIN_READS = 16;

/** What a job runs, and for whom. */
export interface JobSpec {
  kind: Kind;
  argv: readonly string[];
  /** The working directory. This process's when absent. */
  cwd?: string;
  /** Exactly the variables the command sees, before the job's own. This process's when absent. */
  env?: Record<string, string | undefined>;
  /** The command as a list shows it. The argv's words joined when absent. */
  command?: string;
  project?: string | null;
  session?: string | null;
  agent?: string | null;
  tool_use?: string | null;
  /** Gets each chunk of the command's output as it arrives, after the log has it. */
  onOutput?: (chunk: Buffer) => void;
  /** With it, the command's stderr goes to a pipe of its own, and each of its chunks goes here in place of `onOutput`. */
  onError?: (chunk: Buffer) => void;
  /**
   * How long the output may stay open after the command's own process exits, in ms. Without it, the
   * runner drains at most `DRAIN_READS` reads.
   */
  lingerMs?: number;
  /** The size of each of the log's parts, `LOG_PART` when absent. */
  logPart?: number;
  /** Keeps the job's windows off the screen while it runs. */
  hide?: boolean;
  /** The command's stdin, fed as the pipe takes it and then closed. `NUL` when absent. */
  stdin?: string;
}

/** A job this process runs. */
export interface Job {
  id: string;
  dir: string;
  record: JobRecord;
  tree: Tree;
  /** The command's exit code, once its exit is written. */
  done: Promise<number>;
  /** Whether a process the command started held its output open past `lingerMs`, once `done` resolves. */
  held: boolean;
}

/** Starts `spec` as a recorded job of `store`. Throws when the command cannot start. */
export function startJob(store: Store, spec: JobSpec): Job {
  store.sweepDue();
  const started = Date.now();
  const id = store.create(started);
  const dir = store.dir(id);
  let tree: Tree;
  const base = spec.env ?? process.env;
  try {
    const env = { ...base, [PARENT_VAR]: id, [EVENTS_VAR]: join(dir, EVENTS) };
    const run = program(spec.argv, env);
    tree = spawnTree(run.argv, { cwd: spec.cwd, env, line: run.line, pipe: true, split: spec.onError !== undefined, input: spec.stdin !== undefined, name: jobName(id), hide: spec.hide });
  } catch (e) {
    rmSync(dir, { recursive: true, force: true });
    throw e;
  }
  const parent = process.env[PARENT_VAR];
  const loop = base[LOOP_VAR];
  const record: JobRecord = {
    kind: spec.kind,
    project: spec.project ?? null,
    cwd: spec.cwd ?? process.cwd(),
    command: spec.command ?? spec.argv.join(" "),
    session: spec.session ?? null,
    agent: spec.agent ?? null,
    tool_use: spec.tool_use ?? null,
    runner: process.pid,
    root: tree.pid,
    started,
    ...(parent ? { parent } : {}),
    ...(loop ? { loop } : {}),
  };
  let log: LogWriter;
  try {
    writeState(join(dir, RECORD), record);
    log = new LogWriter(dir, spec.logPart ?? LOG_PART);
  } catch (e) {
    tree.kill();
    tree.close();
    throw e;
  }
  // A watcher that cannot start leaves the windows shown, and the command still runs.
  let watcher: WindowWatcher | null = null;
  if (spec.hide) {
    try {
      watcher = tree.watchWindows();
    } catch {}
  }
  const job: Job = { id, dir, record, tree, done: Promise.resolve(0), held: false };
  job.done = pump(tree, log, spec).then(([code, held]) => {
    job.held = held;
    const ended = Date.now();
    writeState(join(dir, EXIT), { code, ended } satisfies JobExit);
    if (record.project !== null) remember(record.project, dir, code, ended);
    return code;
  }).finally(() => {
    watcher?.stop();
    log.close();
    tree.close();
  });
  return job;
}

/** Records the green and red items of the job in `dir`, which ended with `code` at `ended`, in the history of `project`. */
function remember(project: string, dir: string, code: number, ended: number): void {
  try {
    const fold = Fold.read(join(dir, EVENTS));
    fold.finish(code, ended);
    recordRuns(project, runsOf(fold), ended);
  } catch {}
}

/**
 * Moves the tree's output into `log` until the command's own process exits, then drains it as `spec`
 * says. Gives the command's code, and whether its output stayed open past `spec.lingerMs`.
 */
async function pump(tree: Tree, log: LogWriter, spec: JobSpec): Promise<[number, boolean]> {
  const pipes = [{ read: () => tree.read(), to: spec.onOutput, open: true }];
  if (spec.onError) pipes.push({ read: () => tree.readErr(), to: spec.onError, open: true });
  // The stdin left to feed. The pipe never blocks, so each pass writes what it has room for.
  let input = spec.stdin === undefined ? null : Buffer.from(spec.stdin);
  const feed = () => {
    if (input === null) return;
    const put = input.length === 0 ? 0 : tree.write(input);
    if (put === null) input = null;
    else input = input.subarray(put);
    if (input !== null && input.length === 0) {
      tree.closeInput();
      input = null;
    }
  };
  const take = (): boolean => {
    feed();
    let took = false;
    for (const p of pipes.filter((p) => p.open)) {
      const chunk = p.read();
      if (chunk === null) p.open = false;
      if (chunk === null || chunk.length === 0) continue;
      log.write(chunk);
      p.to?.(chunk);
      took = true;
    }
    return took;
  };
  for (;;) {
    if (take()) {
      await Bun.sleep(0);
      continue;
    }
    const code = tree.exitCode();
    if (code === null) {
      await Bun.sleep(PUMP_MS);
      continue;
    }
    if (spec.lingerMs === undefined) {
      // A process the command left running may write on, so the drain takes a bounded number of reads.
      for (let n = 0; n < DRAIN_READS && take(); n++) {}
      return [code, false];
    }
    const until = Date.now() + spec.lingerMs;
    while (pipes.some((p) => p.open) && Date.now() < until) {
      if (!take()) await Bun.sleep(PUMP_MS);
    }
    return [code, pipes.some((p) => p.open)];
  }
}
