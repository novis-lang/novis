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
// `RunState` is `.loop/run.json`, what one turn leaves for the next: `index` names the logs and only goes up,
// and `served` is what `--max-sessions` counts. Its `judge` is a served session no sweep has judged yet, because the session
// changed driver code the turn had imported: `driverFiles` and `driverChanged` find that, and the next
// turn, a fresh process, serves no session and judges that one with the code it committed.

import { createHash } from "node:crypto";
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { say } from "./console.ts";

/** The environment variable that names the run; `driver/respawn.ts` sets it for every turn. */
export const RUN_ENV = "NOVIS_LOOP_RUN";
/** The exit code that asks `driver/respawn.ts` for the next turn. */
export const AGAIN = 75;

export const RUNDIR = ".loop";
export const LOGDIR = ".loop/logs";

export interface LaunchOptions {
  model: string;
  permissionMode: string;
  effort?: string;
  /** A session id whose transcript this launch rejoins, when its stream dropped mid-answer. */
  resume?: string;
}

/** The command line of one session, after the executable. */
export function claudeArgs(o: LaunchOptions): string[] {
  const args = ["-p", "--input-format", "stream-json", "--model", o.model, "--permission-mode", o.permissionMode, "--output-format", "stream-json", "--verbose"];
  if (o.resume) args.push("--resume", o.resume);
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

/** The running child as `onStart` gets it: its pid, a line down its stdin, and whether stdin is still open. */
export interface Started {
  pid: number;
  write: (line: string) => boolean;
  open: () => boolean;
}

/**
 * Runs one session to its end. `exe` is the command that stands for `claude`, which a test replaces with
 * a recorded stream. `onStart` is called once the child exists, which is how the console's keys reach it.
 */
export async function launch(
  exe: string[],
  o: LaunchOptions,
  opening: string,
  log: string,
  pack: { bytes: number; goal: string },
  onEvent: (e: Record<string, any>) => void,
  extraEnv: Record<string, string> = {},
  onStart?: (child: Started) => void,
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
  onStart?.({
    pid: child.pid,
    write: (line) => {
      if (!open) return false;
      try {
        child.stdin.write(line);
        child.stdin.flush();
        return true;
      } catch {
        return false;
      }
    },
    open: () => open,
  });
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
  /** A session the last turn served and left for this one to judge, or `{}`. */
  judge?: Judge | Record<string, never>;
  [other: string]: unknown;
}

/** What a turn that judges an earlier turn's session needs of it. */
export interface Judge {
  index: number;
  line: string;
  commits: number;
  base: string;
}

/** `state.judge` when it names a session, and null when there is none to judge. */
export function pendingJudge(state: RunState): Judge | null {
  const j = state.judge as Partial<Judge> | undefined;
  if (!j || typeof j.index !== "number") return null;
  return { index: j.index, line: String(j.line ?? ""), commits: Number(j.commits) || 0, base: String(j.base ?? "") };
}

/**
 * Every `.ts` file under `tools/` this process has imported, with the digest of its bytes. This is the
 * code a sweep run by this process judges with. A tool the sweep starts as a subprocess reads its code off
 * disk when it starts, and is never stale.
 */
export function driverFiles(root = ROOT): Map<string, string> {
  const tools = join(root, "tools") + sep;
  const out = new Map<string, string>();
  for (const path of Object.keys(require.cache)) {
    if (!path.endsWith(".ts") || !path.startsWith(tools)) continue;
    try {
      out.set(path, createHash("sha256").update(readFileSync(path)).digest("hex"));
    } catch {
      // A module deleted since it was imported is not one a digest can be taken of now either.
    }
  }
  return out;
}

/**
 * The files in `before`, a `driverFiles` snapshot, whose bytes differ now, as sorted paths from the root.
 * A non-empty answer after a session means this process would judge it with the code the session
 * replaced, so a session that fixed the driver's own verdict would fail the same check again.
 */
export function driverChanged(before: Map<string, string>, root = ROOT): string[] {
  const changed: string[] = [];
  for (const [path, digest] of before) {
    let now = "";
    try {
      now = createHash("sha256").update(readFileSync(path)).digest("hex");
    } catch {
      // Deleted: a change like any other.
    }
    if (now !== digest) changed.push(relative(root, path).replace(/\\/g, "/"));
  }
  return changed.sort();
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
  say(line, undefined, true);
}
