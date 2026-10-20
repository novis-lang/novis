// The seam between the loop and an agent CLI: an adapter builds the command line, writes the stdin
// lines, turns each stdout line into normalized events and classifies how a session ended. It ports
// `crates/looper-agent/src/lib.rs`.
//
// - `AgentAdapter` is the whole contract. The loop never reads an agent's own JSON. It sees only
//   `AgentEvent`s, and a `SessionEnd` becomes one `ExitClass`.
// - `Claude` in `claude.ts` is the only real adapter. Its head comment holds the command line, the stdin
//   lines, which stream lines become which events, and the classification rules.
// - `StandIn` in `stand-in.ts` runs that file as a program with Claude's command line plus `--script
//   <file>`. It speaks Claude's stream-json, so it is judged by Claude's own parser and classifier.
//   No test runs `claude`.
//
// The session owns spawning the command, reading stdout one line at a time with each line capped at
// `MAX_LINE`, the trackers, the budgets, the stdin gate, and whether a session it killed itself is
// classified at all.
//
// Parsing holds one line and the events it yields, nothing across lines. Every summary is one line of
// at most `SUMMARY_CHARS` characters, and a result's text is cut to `RESULT_CHARS`. A text event and a
// tool call's input carry what the line carried, so `MAX_LINE` bounds them.

/** The longest stream line the session passes to `parse`. A longer line is cut there and recorded as truncated. */
export const MAX_LINE = 8 * 1024 * 1024;

/** The most characters an event summary holds. */
export const SUMMARY_CHARS = 200;

/** The most characters of a result's final text that `ResultEvent.text` keeps. */
export const RESULT_CHARS = 4096;

/** An agent CLI, as the loop drives it. */
export interface AgentAdapter {
  /** The command for one session. The session pipes its stdin, stdout and stderr. */
  command(opts: LaunchOptions): AgentCommand;
  /** The first stdin line: the prompt with the orientation pack behind it. */
  opening(prompt: string, pack: string): string;
  /** A later stdin line: a `say`, the hand-off prompt or the resume prompt. */
  inject(text: string): string;
  /**
   * The stdin lines that stop the running turn and send `text` as the next user message, in the same
   * session. `n` numbers the session's interrupts, so each request has an id of its own.
   */
  interrupt(n: number, text: string): string;
  /** The events one stdout line carries. A blank line yields none, and any other line yields at least one. */
  parse(line: string | Uint8Array): AgentEvent[];
  /** How a session ended. */
  classify(end: SessionEnd): ExitClass;
}

/** One session's command: its words, its working directory, and the variables it adds to the loop's own. */
export interface AgentCommand {
  argv: string[];
  cwd: string;
  env: Record<string, string>;
}

/** What one session is launched with. */
export interface LaunchOptions {
  /** The tree the session works in. */
  cwd: string;
  model: string;
  permissionMode: string;
  /** The tools the session may call. */
  tools: string[];
  effort?: string;
  /** The agent's session id to rejoin after a dropped stream. */
  resume?: string;
  /** The file that tools append the paths they write to, passed as `LOOPER_WRITTEN`. */
  written?: string;
  /** The loop the session runs in, passed as `LOOPER_LOOP`. */
  loop?: string;
  /** The project root that holds the loop's folder, passed as `LOOPER_ROOT`. A side loop's `cwd` is its lane. */
  root?: string;
  /** The goal the session works on, passed as `LOOPER_GOAL`. */
  goal?: string;
}

/** One normalized stream event. */
export type AgentEvent =
  | { kind: "init"; sessionId: string; model: string }
  /** A text block of the main conversation or of a subagent. */
  | { kind: "text"; text: string }
  | { kind: "tool_use"; id: string; name: string; input: unknown }
  /** `output` is the background task the call started and the file its output goes to, when it started one. */
  | { kind: "tool_result"; id: string; isError: boolean; summary: string; output: TaskOutput | null }
  /** The token counts of one main-conversation message. A subagent's messages yield none, because its context is not the session's. */
  | { kind: "usage"; usage: Usage }
  | { kind: "rate_limit"; limit: RateLimit }
  /** The background tasks live right now, the whole set. */
  | { kind: "background"; tasks: Task[] }
  | { kind: "result"; result: ResultEvent }
  /** A line this adapter does not act on, by its kind, such as `system/thinking_tokens` or `tool_progress`. */
  | { kind: "other"; other: string }
  /** A line that is not JSON. */
  | { kind: "unparsed" };

/** One background task, as the agent named it. A kind or a description the agent left out is empty. */
export interface Task {
  id: string;
  /** Such as `local_bash`. */
  kind: string;
  /** One line of at most `SUMMARY_CHARS` characters. */
  description: string;
}

/** The file a background task writes its output to, as the call that started the task named it. */
export interface TaskOutput {
  task: string;
  path: string;
}

/** The token counts of one message. */
export interface Usage {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
}

/** One rate-limit report of the account. */
export interface RateLimit {
  /** `allowed`, `allowed_warning` or `rejected`. */
  status: string;
  /** The window that applies, such as `five_hour` or `seven_day`. */
  kind: string;
  /** Unix seconds. */
  resetsAt: number;
  /** Utilization from 0 to 1 of the five-hour and seven-day windows, when reported. */
  fiveHour: number | null;
  sevenDay: number | null;
}

/** The `result` that closes a turn. */
export interface ResultEvent {
  sessionId: string;
  /** `success` or an `error_*` subtype. */
  subtype: string;
  isError: boolean;
  /** Why the turn ended, such as `completed` or `api_error`. */
  terminalReason: string;
  /** The HTTP status of an API error, when there was one. */
  apiErrorStatus: number | null;
  numTurns: number;
  durationMs: number;
  costUsd: number;
  /** The whole session's tokens. */
  usage: Usage;
  /** The final text, cut to `RESULT_CHARS`. */
  text: string;
}

/** What the session knows when its process has exited. */
export interface SessionEnd {
  /** The exit code, or null when the process ended without one. */
  code: number | null;
  /** The last `result` of the stream. */
  result: ResultEvent | null;
  /** The last rate-limit report of the stream. */
  limit: RateLimit | null;
  /** Unix seconds. */
  now: number;
}

/**
 * How a session ended, and so what the loop does next.
 *
 * - `ok`: the agent exited cleanly. Whether the work is good is the judge's question, not this one.
 * - `wall`: the account hit its usage limit. `resetsAt` is unix seconds, or null when the agent said
 *   so only in words and the loop waits blind.
 * - `overload`: the API was overloaded (HTTP 529), so the loop backs off and runs again.
 * - `dropped`: the stream dropped mid-answer, so the loop rejoins the same session with `--resume`.
 * - `failed`: any other failure.
 */
export type ExitClass =
  | { kind: "ok" }
  | { kind: "wall"; resetsAt: number | null }
  | { kind: "overload" }
  | { kind: "dropped" }
  | { kind: "failed" };

/** The context a message fills: every input token, cached or not, plus the output. */
export function context(u: Usage): number {
  return u.input + u.output + u.cacheRead + u.cacheWrite;
}

/** Whether `limit` is a usage wall still standing at `now`, in unix seconds. */
export function walls(limit: RateLimit, now: number): boolean {
  return limit.status === "rejected" && limit.resetsAt > now;
}

/** The kind the event index records. */
export function kind(e: AgentEvent): string {
  return e.kind === "other" ? e.other : e.kind;
}

/** One line for the event index and the live view. */
export function summary(e: AgentEvent): string {
  let s: string;
  switch (e.kind) {
    case "init":
      s = `${e.sessionId} (${e.model})`;
      break;
    case "text":
      s = e.text.trim();
      break;
    case "tool_use":
      s = `${e.name} ${toolSummary(e.name, e.input)}`;
      break;
    case "tool_result":
      s = e.summary;
      break;
    case "usage":
      s = `context ${context(e.usage)}`;
      break;
    case "rate_limit":
      s = `${e.limit.status} ${e.limit.kind} resets ${e.limit.resetsAt}`;
      break;
    case "background":
      s = e.tasks.map((t) => t.id).join(" ");
      break;
    case "result":
      s = `${e.result.subtype} ${e.result.numTurns} turns`;
      break;
    case "other":
      s = e.other;
      break;
    case "unparsed":
      s = "";
      break;
  }
  return firstLine(s, SUMMARY_CHARS);
}

/** The id of the tool call a call or its result belongs to, which the event index records. */
export function tool(e: AgentEvent): string | null {
  return (e.kind === "tool_use" || e.kind === "tool_result") && e.id !== "" ? e.id : null;
}

/** What a tool call is about, such as a shell call's command or a file tool's path, in one line of at most `SUMMARY_CHARS` characters. */
export function toolArg(e: AgentEvent): string | null {
  return e.kind === "tool_use" ? firstLine(toolSummary(e.name, e.input), SUMMARY_CHARS) : null;
}

/** The path a file-writing tool call targets, as the agent gave it. */
export function written(e: AgentEvent): string | null {
  if (e.kind !== "tool_use" || !["Write", "Edit", "MultiEdit", "NotebookEdit"].includes(e.name)) return null;
  const input = record(e.input);
  for (const k of ["file_path", "notebook_path"]) if (typeof input[k] === "string") return input[k];
  return null;
}

/** What a tool call is about, in one line: a shell call's command, a file tool's path, a search's pattern. */
function toolSummary(name: string, input: unknown): string {
  const fields = record(input);
  const field = (k: string) => (typeof fields[k] === "string" ? fields[k] : null);
  let byName: string | null = null;
  if (name === "Bash" || name === "PowerShell") byName = field("command");
  else if (name === "Grep") {
    const pattern = field("pattern");
    const path = field("path");
    byName = pattern !== null && path !== null ? `${pattern} in ${path}` : pattern;
  } else if (name === "Glob") byName = field("pattern");
  else if (name === "Agent" || name === "Task") byName = field("description") ?? field("prompt");
  return byName ?? field("file_path") ?? field("notebook_path") ?? JSON.stringify(input ?? null);
}

/** The first non-blank line of `s`, cut to `max` characters. */
export function firstLine(s: string, max: number): string {
  const line = s.split("\n").map((l) => l.replace(/\r$/, "")).find((l) => l.trim() !== "") ?? "";
  return Array.from(line.trimEnd()).slice(0, max).join("");
}

/** `v`'s fields when it is a JSON object, and none otherwise. */
export function record(v: unknown): Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v) ? (v as Record<string, unknown>) : {};
}

/** `v` when it is a string, and `""` otherwise. */
export function str(v: unknown): string {
  return typeof v === "string" ? v : "";
}
