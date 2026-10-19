// The stand-in: a scripted agent that speaks Claude Code's stream-json, so the loop can be tested end to
// end without `claude`. It ports `crates/looper-agent/src/bin/stand-in.rs`. `StandIn` is its adapter.
// It launches this file with Bun, with Claude's whole command line plus `--script <file>`. Of that
// line the program reads only `--script`, `--resume`, `--model` and `--effort`.
//
// # How it runs
//
// Like Claude, it waits for the first stdin line. Then it emits `system/init` with its session id,
// which is the `--resume` id or `stand-in-<pid>`, and plays the script one step at a time. When the
// script runs out it stays open until stdin closes. It answers every later stdin line with a turn of
// its own: a text `heard: <text>` and a `result`. Then it exits 0.
//
// An interrupt request on stdin gets Claude's `control_response`. During a `work` step it also stops
// the step as Claude stops a tool call, and the script ends there. So the user line behind the request
// is answered next. Anywhere else there is no turn to stop, and the request gets only its answer.
//
// # The script
//
// One step per line: a verb, a space, and the rest of the line as its argument. Blank lines and lines
// starting with `#` are skipped. A line `--- resume` splits the script. The steps above it run on a
// fresh launch, and the steps below it when the launch carries `--resume`. In a step's text, `\n` is a
// newline. Paths are relative to the working directory, and a tool call reports them absolute, as
// Claude does.
//
// | Step | What it does |
// |---|---|
// | `text <words>` | an assistant text block |
// | `context <n>` | an assistant message whose usage fills `n` tokens of context |
// | `tool <name> <command>` | a tool call with `{"command": …}` and its result, and nothing else |
// | `write <path> <text>` | a `Write` tool call; the file is created with `text` |
// | `append <path> <text>` | an `Edit` tool call; `text` is appended to the file |
// | `report <path> <text>` | the file is written with no tool call, and its path is appended to `LOOPER_WRITTEN` |
// | `commit <message>` | a `Bash` tool call that commits every path written so far and not yet committed |
// | `listen [ms]` | waits for the next stdin line (30 s by default) and answers `heard: <text>`, or `heard nothing` |
// | `opening <words>` | says `opening has: <words>` when the first stdin line holds them, and `opening lacks: <words>` when not |
// | `launch` | says `launch: model <model>, effort <effort>`, with `none` for an effort not given |
// | `work <ms>` | a `Bash` call that runs that long, or until an interrupt; a user line meanwhile is answered after the script |
// | `rate-limit <status> <seconds> [kind]` | a `rate_limit_event` with that status, resetting that many seconds from now |
// | `background [id ...]` | `system/background_tasks_changed` with those tasks live, each a `local_bash` described `task <id>` |
// | `start <id> <path>` | a `Bash` call whose result says it started background task `id`, its output going to `path` |
// | `result [kind]` | a `result`: `success` (the default), `error`, `dropped`, `overload`, `wall` or `interrupted` |
// | `huge <bytes>` | an assistant text block of that many bytes |
// | `huge-result <bytes>` | a `Bash` tool call whose result is that many bytes |
// | `burst <n>` | `n` assistant text blocks, each one short line |
// | `spawn` | starts a child stand-in with `--sleep 600000` and says `child <pid>` |
// | `run <program> [arg ...]` | a `Bash` call that runs the program with `CLAUDE_CODE_SESSION_ID` set, waits for it, and gives its output |
// | `sleep <ms>` | waits |
// | `exit <code>` | exits at once with that code |
//
// A step that fails, such as a commit git refuses, ends the stand-in with exit code 3 and the reason on
// stderr.
//
// # The bounds it keeps
//
// stdin lines wait in a queue of `STDIN_BOUND` lines, and reading stops while it is full. A `work`
// step holds at most `STDIN_BOUND` user lines for later and drops the rest.

import { appendFileSync, mkdirSync, writeFileSync, writeSync } from "node:fs";
import { dirname, resolve } from "node:path";
import type { AgentAdapter, AgentCommand, AgentEvent, ExitClass, LaunchOptions, SessionEnd } from "./adapter.ts";
import { classify, commandLine, interruptLines, parse, userLine } from "./claude.ts";

/** How many stdin lines wait for the script before reading stops. */
const STDIN_BOUND = 64;

/** How long `listen` waits when the step names no time, in ms. */
const LISTEN_DEFAULT = 30_000;

/** The scripted stand-in: Claude's command line run by `program`, plus `--script <file>`. */
export class StandIn implements AgentAdapter {
  /** `program` is this file run by the running Bun unless a test names another. */
  constructor(
    readonly script: string,
    readonly program: string[] = [process.execPath, import.meta.path],
  ) {}

  command(opts: LaunchOptions): AgentCommand {
    const cmd = commandLine(this.program, opts);
    cmd.argv.push("--script", this.script);
    return cmd;
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

interface Args {
  script?: string;
  resume?: string;
  model: string;
  effort?: string;
  sleep?: number;
}

function args(argv: string[]): Args {
  const out: Args = { model: "" };
  for (let i = 0; i < argv.length; i++) {
    const next = () => argv[++i];
    switch (argv[i]) {
      case "--script":
        out.script = next();
        break;
      case "--resume":
        out.resume = next();
        break;
      case "--model":
        out.model = next() ?? "";
        break;
      case "--effort":
        out.effort = next();
        break;
      case "--sleep": {
        const ms = Number(next());
        if (Number.isFinite(ms)) out.sleep = ms;
        break;
      }
    }
  }
  return out;
}

/** The steps one launch plays: those above `--- resume`, or with `resume` those below it. */
function steps(script: string, resume: boolean): string[] {
  const lines = script.split("\n").map((l) => l.trim());
  const split = lines.indexOf("--- resume");
  const fresh = split < 0 ? lines : lines.slice(0, split);
  const chosen = resume ? (split < 0 ? [] : lines.slice(split + 1)) : fresh;
  return chosen.filter((l) => l !== "" && !l.startsWith("#"));
}

/** One stdin line: a user message's text, or an interrupt request's id. */
type Input = { text: string } | { interrupt: string };

/** Each stdin line, read as it comes, so a step can wait for one with a deadline. */
class Stdin {
  private queue: Input[] = [];
  private closed = false;
  private wake: (() => void) | null = null;
  private room: (() => void) | null = null;

  constructor() {
    void this.pump();
  }

  private async pump(): Promise<void> {
    const decoder = new TextDecoder();
    let buf = "";
    for await (const chunk of Bun.stdin.stream()) {
      buf += decoder.decode(chunk, { stream: true });
      for (let at = buf.indexOf("\n"); at >= 0; at = buf.indexOf("\n")) {
        await this.push(buf.slice(0, at));
        buf = buf.slice(at + 1);
      }
    }
    await this.push(buf);
    this.closed = true;
    this.wake?.();
  }

  private async push(raw: string): Promise<void> {
    const line = raw.replace(/\r$/, "");
    if (line.trim() === "") return;
    let v: Record<string, unknown> = {};
    try {
      v = (JSON.parse(line) ?? {}) as Record<string, unknown>;
    } catch {}
    const request = (v.request ?? {}) as Record<string, unknown>;
    let input: Input;
    if (v.type === "control_request" && request.subtype === "interrupt") {
      input = { interrupt: typeof v.request_id === "string" ? v.request_id : "" };
    } else {
      const content = ((v.message ?? {}) as { content?: { text?: unknown }[] }).content;
      const text = Array.isArray(content) ? content[0]?.text : undefined;
      input = { text: typeof text === "string" ? text : line };
    }
    while (this.queue.length >= STDIN_BOUND) await new Promise<void>((r) => (this.room = r));
    this.queue.push(input);
    this.wake?.();
  }

  /** The next line by `deadline`, a `Date.now()` time, or until stdin closes with none left. Null when neither gives one. */
  async next(deadline?: number): Promise<Input | null> {
    for (;;) {
      const got = this.queue.shift();
      if (got !== undefined) {
        this.room?.();
        return got;
      }
      if (this.closed) return null;
      const left = deadline === undefined ? undefined : deadline - Date.now();
      if (left !== undefined && left <= 0) return null;
      await new Promise<void>((r) => {
        const timer = left === undefined ? undefined : setTimeout(r, left);
        this.wake = () => {
          clearTimeout(timer);
          r();
        };
      });
      this.wake = null;
    }
  }
}

class Session {
  /** Paths written and not yet committed, in the order they were first written. */
  private pending: string[] = [];
  private tools = 0;
  /** User lines that arrived during a `work` step, answered once the script is played. */
  private queued: string[] = [];
  /** Whether an interrupt stopped the script. */
  private interrupted = false;
  private readonly cwd = process.cwd();

  constructor(
    private readonly id: string,
    /** The first stdin line's text. */
    private readonly opening: string,
    private readonly input: Stdin,
    /** The `--model` and `--effort` it was launched with. */
    private readonly model: string,
    private readonly effort: string | undefined,
  ) {}

  /** Plays the steps and then answers stdin until it closes. A number is an `exit` step's code. */
  async play(steps: string[]): Promise<number | null> {
    emit({ type: "system", subtype: "init", session_id: this.id, model: this.model, cwd: this.cwd });
    for (const step of steps) {
      let code: number | null;
      try {
        code = await this.step(step);
      } catch (e) {
        throw new Error(`${step}: ${e instanceof Error ? e.message : String(e)}`);
      }
      if (code !== null) return code;
      if (this.interrupted) break;
    }
    for (const text of this.queued.splice(0)) this.answer(text);
    for (let text = await this.recv(); text !== null; text = await this.recv()) this.answer(text);
    return null;
  }

  /** A turn of its own for a user line: `heard: <text>` and a `result`. */
  private answer(text: string): void {
    this.say(`heard: ${text}`);
    this.result("success");
  }

  /**
   * The next user line within `wait` ms, or until stdin closes without one. An interrupt that comes
   * first has no turn to stop, so it gets only its answer.
   */
  private async recv(wait?: number): Promise<string | null> {
    const deadline = wait === undefined ? undefined : Date.now() + wait;
    for (;;) {
      const got = await this.input.next(deadline);
      if (got === null) return null;
      if ("text" in got) return got.text;
      controlResponse(got.interrupt);
    }
  }

  /** A `Bash` call that runs for `wait` ms, unless an interrupt stops it first. Then it answers as Claude does, and the script stops. */
  private async work(wait: number): Promise<void> {
    const id = this.toolId();
    this.assistant([{ type: "tool_use", id, name: "Bash", input: { command: `work ${wait}` } }]);
    const deadline = Date.now() + wait;
    for (;;) {
      const got = await this.input.next(deadline);
      if (got === null) return this.toolResult(id, "worked", false);
      if ("text" in got) {
        if (this.queued.length < STDIN_BOUND) this.queued.push(got.text);
        continue;
      }
      controlResponse(got.interrupt);
      this.toolResult(id, "The user doesn't want to proceed with this tool use.", true);
      emit({
        type: "user",
        message: { role: "user", content: [{ type: "text", text: "[Request interrupted by user for tool use]" }] },
        parent_tool_use_id: null,
        session_id: this.id,
      });
      this.interrupted = true;
      return this.result("interrupted");
    }
  }

  private async step(step: string): Promise<number | null> {
    const space = step.indexOf(" ");
    const verb = space < 0 ? step : step.slice(0, space);
    const arg = space < 0 ? "" : step.slice(space + 1);
    const text = arg.replaceAll("\\n", "\n");
    switch (verb) {
      case "text":
        this.say(text);
        break;
      case "context":
        this.assistant([], number(arg));
        break;
      case "tool": {
        const [name, command] = splitOnce(arg);
        this.tool(name, { command }, "ok");
        break;
      }
      case "write":
      case "append":
      case "report":
        this.file(verb, text);
        break;
      case "commit":
        this.commit(arg);
        break;
      case "listen": {
        const heard = await this.recv(arg === "" ? LISTEN_DEFAULT : number(arg));
        this.say(heard === null ? "heard nothing" : `heard: ${heard}`);
        break;
      }
      case "work":
        await this.work(number(arg));
        break;
      case "opening":
        this.say(`opening ${this.opening.includes(arg) ? "has" : "lacks"}: ${arg}`);
        break;
      case "launch":
        this.say(`launch: model ${this.model}, effort ${this.effort ?? "none"}`);
        break;
      case "rate-limit":
        this.rateLimit(arg);
        break;
      case "background": {
        const ids = arg.split(/\s+/).filter((id) => id !== "");
        const tasks = ids.map((id) => ({ task_id: id, task_type: "local_bash", description: `task ${id}` }));
        emit({ type: "system", subtype: "background_tasks_changed", tasks, session_id: this.id });
        break;
      }
      case "start": {
        const [task, path] = splitOnce(arg);
        this.start(task, resolve(this.cwd, path));
        break;
      }
      case "result":
        this.result(arg === "" ? "success" : arg);
        break;
      case "huge":
        this.say("x".repeat(number(arg)));
        break;
      case "huge-result": {
        const n = number(arg);
        this.tool("Bash", { command: `print ${n} bytes` }, "x".repeat(n));
        break;
      }
      case "burst":
        for (let i = 1; i <= number(arg); i++) this.say(`burst ${i}`);
        break;
      case "spawn": {
        const child = Bun.spawn([process.execPath, import.meta.path, "--sleep", "600000"], { stdio: ["ignore", "ignore", "ignore"] });
        child.unref();
        this.say(`child ${child.pid}`);
        break;
      }
      case "run": {
        const words = arg.split(/\s+/).filter((w) => w !== "");
        if (words.length === 0) throw new Error("`run` names no program");
        let out: { stdout: Buffer; stderr: Buffer };
        try {
          out = Bun.spawnSync(words, {
            cwd: this.cwd,
            env: { ...process.env, CLAUDE_CODE_SESSION_ID: this.id },
            stdin: "ignore",
            stdout: "pipe",
            stderr: "pipe",
          });
        } catch (e) {
          throw new Error(`cannot run \`${words[0]}\`: ${e instanceof Error ? e.message : String(e)}`);
        }
        this.tool("Bash", { command: arg }, `${out.stdout.toString()}${out.stderr.toString()}`);
        break;
      }
      case "sleep":
        await Bun.sleep(number(arg));
        break;
      case "exit": {
        if (!/^\d+$/.test(arg) || Number(arg) > 255) throw new Error(`\`${arg}\` is not an exit code`);
        return Number(arg);
      }
      default:
        throw new Error(`unknown step \`${verb}\``);
    }
    return null;
  }

  /** `write`, `append` or `report`: changes the file, and says so the way that step does. */
  private file(verb: string, arg: string): void {
    const [path, text] = splitOnce(arg);
    const abs = resolve(this.cwd, path);
    mkdirSync(dirname(abs), { recursive: true });
    if (verb === "append") appendFileSync(abs, text);
    else writeFileSync(abs, text);
    if (!this.pending.includes(path)) this.pending.push(path);
    if (verb === "write") this.tool("Write", { file_path: abs, content: text }, "written");
    else if (verb === "append") this.tool("Edit", { file_path: abs, new_string: text }, "edited");
    else report(abs);
  }

  private commit(message: string): void {
    if (this.pending.length === 0) throw new Error("nothing written to commit");
    git(this.cwd, ["add", "--", ...this.pending]);
    git(this.cwd, ["commit", "-q", "-m", message]);
    this.pending = [];
    this.tool("Bash", { command: `git commit -m "${message}"` }, "committed");
  }

  private rateLimit(arg: string): void {
    const [status = "allowed", secs, kind = "five_hour"] = arg.split(/\s+/).filter((w) => w !== "");
    const resets = now() + (secs === undefined ? 0 : number(secs));
    emit({
      type: "rate_limit_event",
      rate_limit_info: {
        status,
        resetsAt: resets,
        rateLimitType: kind,
        unifiedWindows: { five_hour: { utilization: 0.5, resetsAt: resets } },
      },
      session_id: this.id,
    });
  }

  private result(kind: string): void {
    const kinds: Record<string, [boolean, string, number | null, string]> = {
      success: [false, "completed", null, "done"],
      error: [true, "error", null, "the stand-in failed"],
      dropped: [true, "api_error", null, "the stream dropped"],
      overload: [true, "api_error", 529, "Overloaded"],
      wall: [true, "api_error", 429, "Claude AI usage limit reached"],
      interrupted: [true, "aborted_tools", null, ""],
    };
    const known = Object.hasOwn(kinds, kind) ? kinds[kind] : undefined;
    if (known === undefined) throw new Error(`unknown result \`${kind}\``);
    const [isError, reason, status, text] = known;
    emit({
      type: "result",
      subtype: isError ? "error_during_execution" : "success",
      is_error: isError,
      terminal_reason: reason,
      api_error_status: status,
      num_turns: 1,
      duration_ms: 1,
      total_cost_usd: 0,
      result: text,
      usage: { input_tokens: 0, output_tokens: 0 },
      session_id: this.id,
    });
  }

  private toolId(): string {
    this.tools += 1;
    return `toolu_stand_in_${this.tools}`;
  }

  private tool(name: string, input: unknown, said: string): void {
    const id = this.toolId();
    this.assistant([{ type: "tool_use", id, name, input }]);
    this.toolResult(id, said, false);
  }

  /** A `Bash` call whose result says, in Claude's words, that it started the background task `task`, writing to `output`. */
  private start(task: string, output: string): void {
    const id = this.toolId();
    this.assistant([{ type: "tool_use", id, name: "Bash", input: { command: `task ${task}`, run_in_background: true } }]);
    const said = `Command running in background with ID: ${task}. Output is being written to: ${output}. You will be notified when it completes.`;
    emit({
      type: "user",
      message: { role: "user", content: [{ type: "tool_result", tool_use_id: id, content: said, is_error: false }] },
      parent_tool_use_id: null,
      session_id: this.id,
      tool_use_result: { backgroundTaskId: task },
    });
  }

  private toolResult(id: string, said: string, isError: boolean): void {
    emit({
      type: "user",
      message: { role: "user", content: [{ type: "tool_result", tool_use_id: id, content: said, is_error: isError }] },
      parent_tool_use_id: null,
      session_id: this.id,
    });
  }

  /** An assistant text block. */
  private say(text: string): void {
    this.assistant([{ type: "text", text }]);
  }

  private assistant(content: unknown[], tokens = 0): void {
    emit({
      type: "assistant",
      message: {
        role: "assistant",
        content,
        usage: { input_tokens: tokens, output_tokens: 0, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 },
      },
      parent_tool_use_id: null,
      session_id: this.id,
    });
  }
}

/** Claude's answer to the control request `id`. */
function controlResponse(id: string): void {
  emit({ type: "control_response", response: { subtype: "success", request_id: id, response: { still_queued: [] } } });
}

/** Writes one stream line at once, so the session sees each event as it happens. */
function emit(event: unknown): void {
  const bytes = Buffer.from(`${JSON.stringify(event)}\n`);
  for (let at = 0; at < bytes.length; ) at += writeSync(1, bytes, at);
}

/** Appends `path` to the file named by `LOOPER_WRITTEN`, as a tool that writes outside Write and Edit does. */
function report(path: string): void {
  const list = process.env.LOOPER_WRITTEN;
  if (list === undefined) throw new Error("LOOPER_WRITTEN is not set");
  appendFileSync(list, `${path}\n`);
}

function git(cwd: string, argv: string[]): void {
  const out = Bun.spawnSync(["git", ...argv], { cwd, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  if (out.exitCode !== 0) throw new Error(`git ${argv.join(" ")}: ${out.stderr.toString().trim()}`);
}

/** `s` split at its first space, with an empty second part when it has none. */
function splitOnce(s: string): [string, string] {
  const at = s.indexOf(" ");
  return at < 0 ? [s, ""] : [s.slice(0, at), s.slice(at + 1)];
}

function number(s: string): number {
  if (!/^\d+$/.test(s.trim())) throw new Error(`\`${s}\` is not a number`);
  return Number(s.trim());
}

function now(): number {
  return Math.floor(Date.now() / 1000);
}

async function main(): Promise<number> {
  const a = args(process.argv.slice(2));
  if (a.sleep !== undefined) {
    await Bun.sleep(a.sleep);
    return 0;
  }
  if (a.script === undefined) {
    process.stderr.write("stand-in: --script <file> is required\n");
    return 2;
  }
  let text: string;
  try {
    text = await Bun.file(a.script).text();
  } catch (e) {
    process.stderr.write(`stand-in: cannot read ${a.script}: ${e instanceof Error ? e.message : String(e)}\n`);
    return 2;
  }
  const input = new Stdin();
  const first = await input.next();
  if (first === null) return 0;
  const opening = "text" in first ? first.text : "";
  const s = new Session(a.resume ?? `stand-in-${process.pid}`, opening, input, a.model, a.effort);
  try {
    return (await s.play(steps(text, a.resume !== undefined))) ?? 0;
  } catch (e) {
    process.stderr.write(`stand-in: ${e instanceof Error ? e.message : String(e)}\n`);
    return 3;
  }
}

if (import.meta.main) void main().then((code) => process.exit(code));
