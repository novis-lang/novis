// The Claude Code adapter. It ports `crates/looper-agent/src/claude.rs`.
//
// # The command line
//
// The projects' drivers all launch the same way, and so does the kit:
//
//     claude -p --input-format stream-json --output-format stream-json --verbose
//            --model M --permission-mode P --tools T1,T2,... --strict-mcp-config
//            --settings {"autoMemoryEnabled":false} [--effort E] [--resume ID]
//
// It runs in the session's tree with `LOOPER_AGENT=1`, so a looper command the agent runs asks as
// `agent`. It also gets `LOOPER_WRITTEN` when the session gives a written-paths file. `LOOPER_LOOP` and
// `LOOPER_GOAL` name the loop and the goal, which the guard records in a claim. `LOOPER_ROOT` names the
// project root that holds the loop's folder, so `decision` finds it from a side loop's lane.
// `LOOPER_SIDE_GOAL` names the goal of a side loop in the project root, which every looper command the
// session runs then reads as the live goal. Claude
// reads one JSON user message per stdin line. It stays running until stdin closes, so a message sent
// after a `result` starts another turn.
//
// # The stdin lines
//
// Every line is `{"type":"user","message":{"role":"user","content":[{"type":"text","text":…}]}}`. The
// opening's text is the prompt, a newline, and the orientation pack.
//
// # An interrupt
//
// An interrupt is two lines written together: a control request, then the operator's prompt as a user
// line.
//
//     {"type":"control_request","request_id":"looper-interrupt-N","request":{"subtype":"interrupt"}}
//     {"type":"user","message":{"role":"user","content":[{"type":"text","text":…}]}}
//
// Claude honours it in the same session, so the kit never kills the session to resume it. Against
// `claude` 2.1.293, with the request sent while a turn was running, Claude answered:
//
// 1. `{"type":"control_response","response":{"subtype":"success","request_id":…}}`.
// 2. When a tool call was running: its `tool_result`, with `is_error` set and the text "The user
//    doesn't want to proceed with this tool use". Then a user text line `[Request interrupted by user
//    for tool use]`. When the model was answering: only a user text line `[Request interrupted by
//    user]`.
// 3. A `result` for the interrupted turn: `error_during_execution`, `is_error` set, terminal reason
//    `aborted_tools` or `aborted_streaming`.
// 4. A new `system/init` with the same session id, and the turn that answers the prompt, ending in its
//    own `result`.
//
// So the session must not close stdin on the interrupted turn's `result`. The recorded stream is
// `kit/fixtures/streams/session-interrupt.ndjson`.
//
// # The stream lines
//
// | Line | Events |
// |---|---|
// | `system/init` | `init` |
// | `system/background_tasks_changed` | `background` with each live task's id, `task_type` and `description` |
// | `assistant` | `usage` when it is the main conversation's, then `text` and `tool_use` per content block |
// | `user` | `tool_result` per tool-result block, with the task's output file when the call started a background task |
// | `rate_limit_event` | `rate_limit` |
// | `result` | `result` |
// | anything else | `other` with its `type/subtype` |
// | not JSON | `unparsed` |
//
// A message with a `parent_tool_use_id` is a subagent's. It yields its text and tool calls, so a
// subagent's writes are still the session's. It yields no `usage`, because a subagent's context is its
// own.
//
// A `user` line whose `tool_use_result` names a `backgroundTaskId` started that task. Its result text
// names the task's output file after `Output is being written to: `. No field carries that path, so the
// adapter takes it from the text, and a result worded otherwise names no file.
//
// # Classification
//
// In this order, the first that holds:
//
// 1. **Wall:** the last rate-limit report is `rejected` and resets in the future. Failing that, the
//    process exited non-zero and the result says `usage limit reached` or `rate_limit_error`. That is a
//    wall with no known reset.
// 2. **Overload:** the result's API error status is 529.
// 3. **Dropped:** the result is an error with terminal reason `api_error` and no API status. That is
//    how Claude reports a stream cut mid-answer.
// 4. **Failed:** the process exited non-zero, or without a code.
// 5. **Ok** otherwise.

import { SIDE_GOAL_VAR } from "../goal/model.ts";
import {
  type AgentAdapter,
  type AgentCommand,
  type AgentEvent,
  type ExitClass,
  firstLine,
  type LaunchOptions,
  RESULT_CHARS,
  type RateLimit,
  type ResultEvent,
  record,
  type SessionEnd,
  SUMMARY_CHARS,
  str,
  type TaskOutput,
  type Usage,
  walls,
} from "./adapter.ts";

/** What a tool result that started a background task says before the task's output file. */
const OUTPUT_PHRASE = "Output is being written to: ";

/** The Claude Code CLI, run as `program`. */
export class Claude implements AgentAdapter {
  constructor(readonly program = "claude") {}

  command(opts: LaunchOptions): AgentCommand {
    return commandLine([this.program], opts);
  }

  opening(prompt: string, pack: string): string {
    return userLine(`${prompt}\n${pack}`);
  }

  inject(text: string): string {
    return userLine(text);
  }

  interrupt(n: number, text: string): string {
    return interruptLines(n, text);
  }

  parse(line: string | Uint8Array): AgentEvent[] {
    return parse(line);
  }

  classify(end: SessionEnd): ExitClass {
    return classify(end);
  }
}

/** Claude's command line for one session, run by the words of `program`. */
export function commandLine(program: string[], opts: LaunchOptions): AgentCommand {
  const argv = [...program, "-p", "--input-format", "stream-json", "--output-format", "stream-json"];
  argv.push("--verbose", "--model", opts.model, "--permission-mode", opts.permissionMode);
  argv.push("--tools", opts.tools.join(","), "--strict-mcp-config", "--settings", '{"autoMemoryEnabled":false}');
  if (opts.effort !== undefined) argv.push("--effort", opts.effort);
  if (opts.resume !== undefined) argv.push("--resume", opts.resume);
  const env: Record<string, string> = {};
  if (opts.written !== undefined) env.LOOPER_WRITTEN = opts.written;
  if (opts.loop !== undefined) env.LOOPER_LOOP = opts.loop;
  if (opts.root !== undefined) env.LOOPER_ROOT = opts.root;
  if (opts.goal !== undefined) env.LOOPER_GOAL = opts.goal;
  if (opts.sideGoal !== undefined) env[SIDE_GOAL_VAR] = opts.sideGoal;
  env.LOOPER_AGENT = "1";
  return { argv, cwd: opts.cwd, env };
}

/** One stdin line carrying `text` as a user message. */
export function userLine(text: string): string {
  const msg = { type: "user", message: { role: "user", content: [{ type: "text", text }] } };
  return `${JSON.stringify(msg)}\n`;
}

/** The interrupt request numbered `n`, then a user line carrying `text`. The head comment says what Claude answers. */
export function interruptLines(n: number, text: string): string {
  const request = { type: "control_request", request_id: `looper-interrupt-${n}`, request: { subtype: "interrupt" } };
  return `${JSON.stringify(request)}\n${userLine(text)}`;
}

const decoder = new TextDecoder();

/** The events one stream line carries. */
export function parse(line: string | Uint8Array): AgentEvent[] {
  const text = typeof line === "string" ? line : decoder.decode(line);
  if (text.trim() === "") return [];
  let v: Record<string, unknown>;
  try {
    v = record(JSON.parse(text));
  } catch {
    return [{ kind: "unparsed" }];
  }
  const ty = str(v.type);
  const sub = str(v.subtype);
  let events: AgentEvent[] = [];
  if (ty === "system" && sub === "init") {
    events = [{ kind: "init", sessionId: str(v.session_id), model: str(v.model) }];
  } else if (ty === "system" && sub === "background_tasks_changed") {
    const tasks = blocks(v.tasks).map((t) => {
      const task = record(t);
      return { id: str(task.task_id), kind: str(task.task_type), description: firstLine(str(task.description), SUMMARY_CHARS) };
    });
    events = [{ kind: "background", tasks }];
  } else if (ty === "assistant") {
    events = assistant(v);
  } else if (ty === "user") {
    const task = str(record(v.tool_use_result).backgroundTaskId);
    for (const b of blocks(record(v.message).content).map(record)) {
      if (b.type !== "tool_result") continue;
      const said = resultText(b.content);
      events.push({
        kind: "tool_result",
        id: str(b.tool_use_id),
        isError: b.is_error === true,
        summary: firstLine(said, SUMMARY_CHARS),
        output: taskOutput(task, said),
      });
    }
  } else if (ty === "rate_limit_event") {
    events = [{ kind: "rate_limit", limit: rateLimit(record(v.rate_limit_info)) }];
  } else if (ty === "result") {
    events = [{ kind: "result", result: result(v) }];
  }
  if (events.length === 0) return [{ kind: "other", other: sub === "" ? ty : `${ty}/${sub}` }];
  return events;
}

/** How a session ended. The rules and their order are this file's head comment. */
export function classify(end: SessionEnd): ExitClass {
  const failed = end.code !== 0;
  if (end.limit !== null && walls(end.limit, end.now)) return { kind: "wall", resetsAt: end.limit.resetsAt };
  if (failed && end.result !== null && saysUsageLimit(end.result)) return { kind: "wall", resetsAt: null };
  const r = end.result;
  if (r !== null) {
    if (r.apiErrorStatus === 529) return { kind: "overload" };
    if (r.isError && r.terminalReason === "api_error" && r.apiErrorStatus === null) return { kind: "dropped" };
  }
  return failed ? { kind: "failed" } : { kind: "ok" };
}

function saysUsageLimit(r: ResultEvent): boolean {
  const text = r.text.toLowerCase();
  return text.includes("usage limit reached") || text.includes("rate_limit_error");
}

/**
 * The output file of the background task `task` a call started, from the result's text. It is what
 * follows `Output is being written to: `, up to the sentence's end. Null without a task or that phrase.
 */
function taskOutput(task: string, text: string): TaskOutput | null {
  const at = text.indexOf(OUTPUT_PHRASE);
  if (task === "" || at < 0) return null;
  const line = (text.slice(at + OUTPUT_PHRASE.length).split("\n")[0] ?? "").replace(/\r$/, "");
  const stop = line.indexOf(". ");
  const path = (stop < 0 ? line : line.slice(0, stop)).trim().replace(/\.+$/, "");
  return path === "" ? null : { task, path };
}

function assistant(v: Record<string, unknown>): AgentEvent[] {
  const msg = record(v.message);
  const out: AgentEvent[] = [];
  const usageOf = msg.usage;
  if ((v.parent_tool_use_id ?? null) === null && typeof usageOf === "object" && usageOf !== null && !Array.isArray(usageOf)) {
    out.push({ kind: "usage", usage: usage(usageOf) });
  }
  for (const b of blocks(msg.content).map(record)) {
    if (b.type === "text" && str(b.text).trim() !== "") out.push({ kind: "text", text: str(b.text) });
    else if (b.type === "tool_use") out.push({ kind: "tool_use", id: str(b.id), name: str(b.name), input: b.input ?? null });
  }
  return out;
}

function usage(u: unknown): Usage {
  const fields = record(u);
  const n = (k: string) => unsigned(fields[k]) ?? 0;
  return {
    input: n("input_tokens"),
    output: n("output_tokens"),
    cacheRead: n("cache_read_input_tokens"),
    cacheWrite: n("cache_creation_input_tokens"),
  };
}

function rateLimit(i: Record<string, unknown>): RateLimit {
  const window = (k: string) => {
    const u = record(record(i.unifiedWindows)[k]).utilization;
    return typeof u === "number" ? u : null;
  };
  return {
    status: str(i.status),
    kind: str(i.rateLimitType),
    resetsAt: Number.isSafeInteger(i.resetsAt) ? (i.resetsAt as number) : 0,
    fiveHour: window("five_hour"),
    sevenDay: window("seven_day"),
  };
}

function result(v: Record<string, unknown>): ResultEvent {
  const said = (x: unknown) => (typeof x === "string" ? x : JSON.stringify(x));
  const head = v.result === undefined || v.result === null ? "" : said(v.result);
  const text = [head, ...blocks(v.errors).map(said)].join("\n");
  const status = unsigned(v.api_error_status);
  return {
    sessionId: str(v.session_id),
    subtype: str(v.subtype),
    isError: v.is_error === true,
    terminalReason: str(v.terminal_reason),
    apiErrorStatus: status !== null && status <= 0xffff ? status : null,
    numTurns: unsigned(v.num_turns) ?? 0,
    durationMs: unsigned(v.duration_ms) ?? 0,
    costUsd: typeof v.total_cost_usd === "number" ? v.total_cost_usd : 0,
    usage: usage(v.usage),
    text: Array.from(text.trim()).slice(0, RESULT_CHARS).join(""),
  };
}

/** A tool result's text: a plain string, or the text blocks of a list. */
function resultText(content: unknown): string {
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return "";
  return content
    .map(record)
    .filter((x) => x.type === "text")
    .map((x) => str(x.text))
    .join("\n");
}

/** `v` when it is a whole number from 0 up, and null otherwise. */
function unsigned(v: unknown): number | null {
  return Number.isSafeInteger(v) && (v as number) >= 0 ? (v as number) : null;
}

function blocks(v: unknown): unknown[] {
  return Array.isArray(v) ? v : [];
}
