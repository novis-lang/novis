// The shell wrapper: Claude Code's shell prefix, run as `bun tools/looper/main.ts sh <invocation>`. It
// ports `crates/looper-sh/src/main.rs`. The harness hands it the whole shell invocation as one
// argument, and it runs `bash -c <invocation>` unchanged, with Git's bash (`shellPath`). Its exit code
// is bash's.
//
// A Bash call (`bashCall`) made in a project, the nearest directory at or above the cwd that holds a
// `looper.toml`, runs as a recorded job of kind `bash` in the project's store (`spawn.ts`):
//
// 1. A call that `heavy.ts` names heavy first asks looper's service for one slot (`slots.ts`). No
//    call asks for more. A wait prints the service's waiting line on stderr. A service that
//    is absent costs nothing, and one that fails prints `looper sh: no heavy slot: <why>`. Either way
//    the call runs without a slot. A call that finds `SLOT_VAR` set takes none.
// 2. It takes the guard's pending entry for its session and command (`pending.ts`), and starts bash in
//    the job's Job Object. The record names the project, the cwd, the command inside the `eval`, the
//    session from `CLAUDE_CODE_SESSION_ID`, and the subagent and tool call from the entry. bash runs
//    with `SLOT_VAR` set to the lease's id when it holds slots. The job's windows are kept off the
//    screen (`spawn.ts`). A project whose `[jobs] hide_windows` is false runs bash with
//    `LOOPER_WINDOWS=show` instead, and a wrapper that inherits that hides nothing.
// 3. It copies bash's output to its own stdout as it arrives, and the job's log keeps it too.
// 4. Once bash exits the job ends, and every process bash left is killed. A job `Store.kill` ended
//    prints who asked on stderr. The lease is given back.
//
// The wrapper reads the start of the Claude Code process `CLAUDE_PID` names once, before it asks or
// starts anything. While it waits or runs, it reads that process again every `CLAUDE_EVERY_MS`. When the
// process has exited, a wait exits with `KILLED_CODE` and a job is killed.
//
// Anything else runs as plain `bash -c` with the wrapper's own stdio, unrecorded: a hook or status line
// command, a call outside a project, and a call whose job cannot be started. The last prints one line,
// `looper sh: not recorded: <why>`, and still runs.
//
// # The bounds it keeps
//
// - The output moves through the job's pipe one chunk at a time (`spawn.ts`).
// - The log keeps its own bound of three parts (`log.ts`).
//
// # Known gaps
//
// - bash's stderr reaches the wrapper's stdout, because a job's tree has one pipe for both.

import { existsSync, statSync, writeSync } from "node:fs";
import { delimiter, dirname, join, resolve } from "node:path";
import { defaultConfig, type Jobs, loadConfig } from "../goal/config.ts";
import { sameProcess, started } from "../os/index.ts";
import { take as heavy } from "./heavy.ts";
import { pendingDir, sweep as sweepPending, take as takePending } from "./pending.ts";
import { type Lease, looperHome, SLOT_VAR, takeSlots } from "./slots.ts";
import { startJob } from "./spawn.ts";
import { KILLED_CODE, storeOf, WINDOWS_VAR } from "./store.ts";

/** How often the wrapper reads whether Claude Code still runs. */
export const CLAUDE_EVERY_MS = 250;

/** The environment the wrapper reads and hands on. */
export type Env = Record<string, string | undefined>;

/** What a shell invocation holds: a Bash call with its command, or one whose `eval` word does not parse. */
export type Call = { command: string } | { raw: true };

/**
 * The Bash call in `invocation`, or null when it is not one. A Bash call's invocation sources the
 * session's snapshot, runs the command as `eval '<command>'`, then writes the cwd for the harness. A
 * hook, status line or MCP server command has no `eval '`.
 */
export function bashCall(invocation: string): Call | null {
  const at = invocation.indexOf(" eval '");
  if (at < 0) return null;
  const command = word(invocation.slice(at + " eval ".length));
  return command === null ? { raw: true } : { command };
}

/** The shell word at the head of `text`, unquoted: `'…'`, `"…"` and `\x` joined until a blank. Null when a quote is not closed. */
function word(text: string): string | null {
  let out = "";
  for (let i = 0; i < text.length; i++) {
    const c = text[i] as string;
    if (c === " " || c === "\t" || c === "\n") return out;
    if (c === "'") {
      const end = text.indexOf("'", i + 1);
      if (end < 0) return null;
      out += text.slice(i + 1, end);
      i = end;
    } else if (c === '"') {
      for (i++; ; i++) {
        if (i >= text.length) return null;
        const d = text[i] as string;
        if (d === '"') break;
        if (d === "\\") {
          i++;
          if (i >= text.length) return null;
          const e = text[i] as string;
          if (e === '"' || e === "\\" || e === "$" || e === "`") out += e;
          else if (e !== "\n") out += `\\${e}`;
        } else out += d;
      }
    } else if (c === "\\") {
      i++;
      if (i >= text.length) return null;
      out += text[i];
    } else out += c;
  }
  return out;
}

function isFile(path: string): boolean {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

/** Each `name` that exists in a directory of `env`'s `PATH`, in `PATH`'s order. */
function onPath(env: Env, name: string): string[] {
  return (env.PATH ?? env.Path ?? "")
    .split(delimiter)
    .filter((d) => d !== "")
    .map((d) => join(d, name))
    .filter(isFile);
}

/** Whether `bash` is WSL's: under the Windows directory `windows`, or in a `WindowsApps` directory. */
export function isWsl(bash: string, windows: string | undefined): boolean {
  const lower = bash.toLowerCase();
  const inWindows = windows !== undefined && lower.startsWith(`${windows.toLowerCase().replace(/\\+$/, "")}\\`);
  return inWindows || dirname(bash).split(/[/\\]/).pop()?.toLowerCase() === "windowsapps";
}

/**
 * The Git Bash that Claude Code runs, found where it looks: `CLAUDE_CODE_GIT_BASH_PATH` when that names
 * a `bash` or `sh` that exists, then `Git\bin\bash.exe` under each Program Files directory, then
 * `bin\bash.exe` in the parent of the directory that holds the `git.exe` on `PATH`.
 */
function gitBash(env: Env): string | undefined {
  const named = env.CLAUDE_CODE_GIT_BASH_PATH;
  const tries: string[] = [];
  if (named !== undefined && /^(bash|sh)(\.exe)?$/i.test(named.split(/[/\\]/).pop() ?? "")) tries.push(named);
  for (const dir of [env.ProgramFiles, env["ProgramFiles(x86)"]]) if (dir !== undefined) tries.push(join(dir, "Git", "bin", "bash.exe"));
  const git = onPath(env, "git.exe")[0];
  if (git !== undefined) tries.push(join(dirname(dirname(git)), "bin", "bash.exe"));
  return tries.find(isFile);
}

/** The bash the wrapper runs: the first on `PATH` that is not WSL's, else Git Bash, else `bash`. */
export function shellPath(env: Env = process.env): string {
  return onPath(env, "bash.exe").find((b) => !isWsl(b, env.SystemRoot)) ?? gitBash(env) ?? "bash";
}

/** The nearest directory at or above `cwd` that holds a `looper.toml`, or null. */
function projectOf(cwd: string): string | null {
  for (let d = resolve(cwd); ; d = dirname(d)) {
    if (existsSync(join(d, "looper.toml"))) return d;
    if (dirname(d) === d) return null;
  }
}

/** Runs `bash -c <args>` with the wrapper's own stdio, and gives its exit code. */
async function plain(bash: string, args: readonly string[], env: Env, cwd: string): Promise<number> {
  try {
    return await Bun.spawn([bash, "-c", ...args], { cwd, env, stdio: ["inherit", "inherit", "inherit"] }).exited;
  } catch (e) {
    process.stderr.write(`looper sh: cannot start ${bash}: ${(e as Error).message}\n`);
    return 127;
  }
}

/** The Claude Code process `CLAUDE_PID` names, with its start, or null when none runs. */
type Claude = { pid: number; at: number } | null;

/** Reads `CLAUDE_PID`'s process once, before the wrapper starts anything, so an exit after that is seen. */
function claudeOf(env: Env): Claude {
  const pid = Number(env.CLAUDE_PID?.trim());
  const at = Number.isInteger(pid) && pid > 0 ? started(pid) : null;
  return at === null ? null : { pid, at };
}

/** Calls `gone` once the `claude` process has exited. Gives the stop of the watch. */
function watchClaude(claude: Claude, gone: () => void): () => void {
  if (claude === null) return () => {};
  const { pid, at } = claude;
  const timer = setInterval(() => {
    if (!sameProcess(pid, at)) {
      clearInterval(timer);
      gone();
    }
  }, CLAUDE_EVERY_MS);
  return () => clearInterval(timer);
}

/** The project's `[jobs]`, or the defaults with the reason on stderr. */
function jobsOf(root: string): Jobs {
  try {
    return loadConfig(root).jobs;
  } catch (e) {
    process.stderr.write(`looper sh: only the built-in heavy commands: ${(e as Error).message}\n`);
    return defaultConfig().jobs;
  }
}

/** Writes `bytes` to this process's stdout. A harness that stopped reading costs only the copy. */
export function out(bytes: Uint8Array): void {
  try {
    for (let at = 0; at < bytes.length; ) at += writeSync(1, bytes.subarray(at));
  } catch {}
}

/** Runs the shell invocation `args` as the head comment says, in `cwd` with `env`, and gives bash's exit code. */
export async function runSh(args: readonly string[], env: Env = process.env, cwd = process.cwd()): Promise<number> {
  const bash = shellPath(env);
  const invocation = args[0];
  const call = args.length > 0 && invocation !== undefined ? bashCall(invocation) : null;
  const root = call === null ? null : projectOf(cwd);
  if (call === null || root === null || invocation === undefined) return plain(bash, args, env, cwd);
  const command = "command" in call ? call.command : invocation;

  const claude = claudeOf(env);
  const jobs = jobsOf(root);
  let lease: Lease | null = null;
  if (env[SLOT_VAR] === undefined || env[SLOT_VAR] === "") {
    const asking = takeSlots(looperHome(env), heavy(command, jobs), { kind: "bash", project: root, command }, (line) =>
      process.stderr.write(`${line}\n`),
    );
    const stop = watchClaude(claude, () => process.exit(KILLED_CODE));
    const asked = await asking;
    stop();
    if ("lease" in asked) lease = asked.lease;
    else if ("failed" in asked) process.stderr.write(`looper sh: no heavy slot: ${asked.failed}\n`);
  }
  const jobEnv = { ...env, ...(lease === null ? {} : { [SLOT_VAR]: lease.id }), ...(jobs.hide_windows ? {} : { [WINDOWS_VAR]: "show" }) };
  const hide = jobEnv[WINDOWS_VAR]?.toLowerCase() !== "show";
  try {
    return await recorded(root, bash, args, command, jobEnv, cwd, lease, hide, claude);
  } catch (e) {
    process.stderr.write(`looper sh: not recorded: ${(e as Error).message}\n`);
    return plain(bash, args, jobEnv, cwd);
  } finally {
    await lease?.release();
  }
}

/** Runs the call as a job of the project at `root`, and gives bash's exit code. Throws, having started nothing, when the job cannot start. */
async function recorded(root: string, bash: string, args: readonly string[], command: string, env: Env, cwd: string, lease: Lease | null, hide: boolean, claude: Claude): Promise<number> {
  const store = storeOf(root);
  if (store.sweepDue() !== null) sweepPending(pendingDir(root));
  const session = env.CLAUDE_CODE_SESSION_ID || null;
  const entry = session === null ? null : takePending(pendingDir(root), session, command);
  const job = startJob(store, {
    kind: "bash",
    argv: [bash, "-c", ...args],
    cwd,
    env,
    command,
    project: root,
    session,
    agent: entry?.agent_id ?? null,
    tool_use: entry?.tool_use_id ?? null,
    onOutput: out,
    hide,
  });
  await lease?.hold(job.id);
  const stop = watchClaude(claude, () => {
    try {
      job.tree.kill(KILLED_CODE);
    } catch {}
  });
  const code = await job.done.finally(stop);
  const killed = store.killed(job.id);
  if (killed) process.stderr.write(`looper: job ${job.id} was killed by ${killed.by}\n`);
  return code;
}
