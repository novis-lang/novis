// One `claude -p` session, and the run state that numbers it. `launch` starts the CLI with stream-json on
// both ends, writes the opening message down its stdin, and hands each event it prints to `onEvent` and
// each line to the session's log, `.loop/logs/<run id>-<index>.log`. The log's first line is a
// `loop_pack` record, the pack's size and the goal, which `loop-stats` fits the pack's cost from.
//
// The opening message is the session prompt with the pack behind it, the order the CLI itself puts an
// argv prompt and a piped stdin in. Stdin stays open until the terminal `result` event, because it is
// the channel a later message to the session would use. The child's environment is this process's with
// `extraEnv` over it, less `NOVIS_LOOP_RUN`: that variable tells a turn it belongs to the run holding
// `.loop/running`, and a session that inherited it would say the same of a `bun nv loop` it typed.
//
// `RunState` is `.loop/run.json`, in the shape `loop.py` writes, so a run continues across the cutover
// with its numbering intact: `index` names the logs and only goes up, and `served` is what
// `--max-sessions` counts.

import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { ROOT } from "../lib/paths.ts";

/** The environment variable that names the run; `tools/respawn.py` sets it for every turn. */
export const RUN_ENV = "NOVIS_LOOP_RUN";
/** The exit code that asks `tools/respawn.py` for the next turn. */
export const AGAIN = 75;

export const RUNDIR = ".loop";
export const LOGDIR = ".loop/logs";

export interface LaunchOptions {
  model: string;
  permissionMode: string;
  effort?: string;
}

/** The command line of one session, after the executable. */
export function claudeArgs(o: LaunchOptions): string[] {
  const args = ["-p", "--input-format", "stream-json", "--model", o.model, "--permission-mode", o.permissionMode, "--output-format", "stream-json", "--verbose"];
  // Only when asked: the flag and the model's own default are not the same thing to the harness.
  if (o.effort) args.push("--effort", o.effort);
  return args;
}

/** The opening user message, as one stream-json line. */
export function openingLine(prompt: string, pack: string): string {
  const text = pack ? `${prompt}\n${pack}` : prompt;
  return `${JSON.stringify({ type: "user", message: { role: "user", content: [{ type: "text", text }] } })}\n`;
}

export interface Launched {
  code: number;
  /** The `session_id` off the `system`/`init` event, or "" when none arrived. */
  sessionId: string;
  /** The terminal `result` event, or null when the stream ended without one. */
  result: Record<string, unknown> | null;
}

/**
 * Runs one session to its end. `exe` is the command that stands for `claude`, which a test replaces with
 * a recorded stream.
 */
export async function launch(
  exe: string[],
  o: LaunchOptions,
  opening: string,
  log: string,
  pack: { bytes: number; goal: string },
  onEvent: (e: Record<string, any>) => void,
  extraEnv: Record<string, string> = {},
): Promise<Launched> {
  mkdirSync(dirname(log), { recursive: true });
  appendFileSync(log, `${JSON.stringify({ type: "loop_pack", ...pack })}\n`);
  const env: Record<string, string | undefined> = { ...process.env, ...extraEnv };
  delete env[RUN_ENV];
  const child = Bun.spawn([...exe, ...claudeArgs(o)], { cwd: ROOT, env, stdin: "pipe", stdout: "pipe", stderr: "inherit" });
  child.stdin.write(opening);
  child.stdin.flush();
  let open = true;
  const close = () => {
    if (!open) return;
    open = false;
    try {
      child.stdin.end();
    } catch {
      // A child that exited first has already closed its end.
    }
  };
  let sessionId = "";
  let result: Record<string, unknown> | null = null;
  const decoder = new TextDecoder();
  let pending = "";
  const take = (line: string) => {
    if (line.trim() === "") return;
    appendFileSync(log, `${line}\n`);
    let e: Record<string, any>;
    try {
      e = JSON.parse(line);
    } catch {
      return;
    }
    if (!sessionId && e.type === "system" && e.subtype === "init") sessionId = String(e.session_id ?? "");
    if (e.type === "result") {
      result = e;
      close();
    }
    onEvent(e);
  };
  for await (const chunk of child.stdout) {
    pending += decoder.decode(chunk, { stream: true });
    let nl: number;
    while ((nl = pending.indexOf("\n")) >= 0) {
      take(pending.slice(0, nl).replace(/\r$/, ""));
      pending = pending.slice(nl + 1);
    }
  }
  take(pending);
  close();
  return { code: await child.exited, sessionId, result };
}

/** `.loop/run.json`: what one turn of a run leaves for the next. */
export interface RunState {
  /** The run's name, `NOVIS_LOOP_RUN`'s value. */
  run: string;
  /** The stamp every log of the run is named with. */
  run_id: string;
  served: number;
  index: number;
  stalls: number;
  [other: string]: unknown;
}

/** The run state for `run`: the file's when it names the same run, and a fresh one otherwise. */
export function loadRun(run: string, root = ROOT): { state: RunState; fresh: boolean } {
  const path = join(root, RUNDIR, "run.json");
  if (existsSync(path)) {
    try {
      const v = JSON.parse(readFileSync(path, "utf8"));
      if (v && v.run === run) return { state: { ...v, served: Number(v.served) || 0, index: Number(v.index) || 0, stalls: Number(v.stalls) || 0 }, fresh: false };
    } catch {
      // An unreadable file starts the run over, which renumbers nothing already logged: the stamp differs.
    }
  }
  const stamp = run.replace(/-\d+$/, "");
  return { state: { run, run_id: stamp, served: 0, index: 0, stalls: 0 }, fresh: true };
}

export function saveRun(state: RunState, root = ROOT): void {
  const path = join(root, RUNDIR, "run.json");
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(state, null, 2)}\n`);
}

/** A run name for a turn started with no `NOVIS_LOOP_RUN`: the local time and this process's id. */
export function runName(now = new Date(), pid = process.pid): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}${p(now.getMonth() + 1)}${p(now.getDate())}-${p(now.getHours())}${p(now.getMinutes())}${p(now.getSeconds())}-${pid}`;
}

/** The ledger, `.loop/log.md`: one line appended. */
export function ledger(line: string, root = ROOT): void {
  const path = join(root, RUNDIR, "log.md");
  mkdirSync(dirname(path), { recursive: true });
  appendFileSync(path, `${line}\n`);
  console.log(line);
}
