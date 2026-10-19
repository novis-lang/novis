// The project's hooks, `[hooks]` in `looper.toml`. It ports `crates/looper-engine/src/hooks.rs`.
//
// `runHooks` runs one event's hooks in order, from the project root, each with the project's `[checks]`
// and the loop's variables. A hook is judged by its exit code alone, and its log is
// `<log dir>/<event>-<n>.log`.
//
// - A hook that fails with `on_fail = "pause"` ends the event, and its reason is the pause's. One with
//   `on_fail = "warn"` adds a warning, and the next hook runs.
// - An attached hook runs as a check does (`runCheck`), as a `hook` job. Its output is kept within
//   `OUTPUT_HALF` at each end, and its whole tree is killed once it exits.
// - A detached hook (`detach = true`) writes its output to its log file, not to a pipe, so a process it
//   leaves running never writes into a closed pipe. The kit waits for the hook's own process alone, and
//   its timeout kills the whole tree. Once it exits, its tree is released and runs on, the project's to
//   manage.
//
// The loop's variables, `LOOPER_GOAL`, `LOOPER_STAGE` and `LOOPER_EXTRA`, are `goalVars` of
// `../cli/orient.ts`, which `check` and `verify` export alone, so a hook sees the same goal inside the
// loop and outside it.
//
// # The bounds it keeps
//
// An event's warnings hold at most one per hook the event has.

import { mkdirSync } from "node:fs";
import { join } from "node:path";
import type { Hook, Hooks } from "../goal/config.ts";
import { emptyCheck } from "../goal/model.ts";
import { spawnTree } from "../os/index.ts";
import { envOf, type Project, runCheck } from "../verify/accept.ts";

/** When a hook runs, by its key in `[hooks]`. */
export type HookEvent = keyof Hooks;

/** What one event's hooks came to. */
export interface Ran {
  /** Why the hook that pauses the loop failed. */
  paused: string | null;
  /** One line for each hook that warned. */
  warnings: string[];
}

/** Runs `event`'s hooks of `hooks` in order from `root`, with `project`, writing their logs to `logDir`. */
export async function runHooks(root: string, event: HookEvent, hooks: Hooks, logDir: string, project: Project): Promise<Ran> {
  const ran: Ran = { paused: null, warnings: [] };
  for (const [i, h] of hooks[event].entries()) {
    const id = `${event}-${i + 1}`;
    const named = `the ${event} hook \`${h.run.join(" ")}\``;
    const why = h.detach
      ? await detached(root, id, h, logDir, project)
      : (await runCheck(root, { ...emptyCheck(), id, run: h.run, timeout: h.timeout }, logDir, project, "hook")).why;
    if (why === "") continue;
    if (h.on_fail === "pause") {
      ran.paused = `${named} ${why}`;
      return ran;
    }
    ran.warnings.push(`${named} ${why}`);
  }
  return ran;
}

/** Runs the detached hook `h` as `id`, and gives why it failed, or empty. */
async function detached(root: string, id: string, h: Hook, logDir: string, project: Project): Promise<string> {
  if (h.run.length === 0) return "cannot run: the argv is empty";
  mkdirSync(logDir, { recursive: true });
  let tree: ReturnType<typeof spawnTree>;
  try {
    tree = spawnTree(h.run, {
      cwd: root,
      env: envOf(project, { ...emptyCheck(), id, run: h.run }),
      log: join(logDir, `${id}.log`),
      hide: project.checks.hide_windows,
    });
  } catch (e) {
    return `cannot run \`${h.run.join(" ")}\`: ${(e as Error).message}`;
  }
  const secs = h.timeout ?? project.checks.timeout;
  let timer: Timer | undefined;
  const timeout = new Promise<null>((done) => {
    timer = setTimeout(() => done(null), secs * 1000);
  });
  const code = await Promise.race([tree.exited(), timeout]);
  clearTimeout(timer);
  if (code === null) {
    tree.kill();
    tree.close();
    return `timed out after ${secs}s`;
  }
  tree.release();
  tree.close();
  return code === 0 ? "" : `exited ${code}, want 0`;
}
