// One session, supervised: the agent runs in a `Tree`, every stream line is recorded as it arrives, and
// the budgets, the timeout, spin, the stdin gate and the operator's controls act on it while it runs. It
// ports `crates/looper-engine/src/session.rs`.
//
// # How a line flows
//
// `serve` polls the tree's stdout and stderr pipes, which never block, and cuts each line at `MAX_LINE`.
// A stdout line is appended raw to the session's `stream.ndjson`, then parsed by the adapter. Each event
// it yields is appended to the session's `events/` log, which gives it the next `seq`, and feeds the
// trackers. Then it goes to `Launch.live`, so every live event is already in the log. A stderr line is
// recorded the same way, wrapped as `{"type":"looper_stderr","text":…}`, and feeds nothing.
//
// An event record is `{seq, ts, kind, summary, tool, offset, len}`. `offset` and `len` address its raw
// line in `stream.ndjson`, so a reader pages the events by `seq` and fetches a raw line on demand.
//
// # The stats
//
// The trackers keep the session's `Stats`: the newest context and the peak, the tool calls, and the
// tokens in and out, where a usage report equal to the one before it is the same message's and counts
// once. They also keep the live background tasks, each stamped when its id is first seen, and the tool
// calls whose results have not come, of which the snapshot names the oldest. They keep the newest call
// and when it ended, and whether a turn runs. A turn's `result` closes every call still open. Each
// snapshot reads the modified time of `Launch.goal` as its `goalStamp`, one file stat, so a client sees
// the goal file change while the session runs.
//
// A tool result that started a background task names the task's output file. Every `TAIL_EVERY` a tick
// stats each live task's file, and reads the last `TAIL_BYTES` of one that changed length. The snapshot
// carries its last `TAIL_LINES` lines with the task (`tail`), so a client sees what a long task prints
// while the agent waits for it.
//
// A tick hands a snapshot to `Launch.stats` when they changed and `STATS_EVERY` has passed since the
// last one, so one goes out at most once a second, and the first tick sends one at once. It is handed
// `null` once the session has ended, whatever ended it.
//
// # What acts on the session
//
// - **Budgets.** At the hard budget the hand-off prompt is injected once. At the kill budget the tree is
//   killed, and without one the session is never killed for its context.
// - **Spin.** The same tool call with the same input `Limits.spinLimit` times in a row kills it.
// - **Timeout.** The session is killed when it runs past `Limits.timeout`.
// - **The stdin gate.** On a `result` stdin is closed, which ends a `-p` session, unless a background
//   task is live. Then it stays open, so the task's report can start a follow-up turn. It is closed once
//   the follow-up's `result` arrives with no task live, `Limits.followUpGrace` after the last task ends
//   with no turn running, or `Limits.backgroundWait` after the `result`, whichever is first.
// - **Controls.** A `say` is written to stdin, or kept in `Served.notes` for the next session's pack
//   when stdin is closed. An `interrupt` is the adapter's interrupt lines while a turn runs, and a say
//   otherwise. The stopped turn's `result` does not close stdin, because the prompt is the next turn.
//   The follow-up grace closes it if that turn never starts. `freeze` and `thaw` suspend and resume the
//   whole tree, and `kill` kills it.
//
// When the agent exits, whatever it left running in its tree is killed, so its pipes close and the
// session ends.
//
// # The bounds it keeps
//
// - One line of at most `MAX_LINE` bytes per stream. A longer line is cut and recorded as `truncated`.
// - One read of a pipe takes at most the OS module's `READ_BYTES`.
// - At most `CONTROL_BOUND` controls wait in `Controls`, and a send past that is refused.
// - At most `WRITTEN_MAX` written paths and `NOTES_MAX` notes are kept. Past that they are counted as
//   dropped.
// - Spin keeps a hash of the last tool call, never the call.
// - A stats snapshot names at most `TASKS_MAX` background tasks and counts the rest. At most `OPEN_MAX`
//   open tool calls are followed, and a call past that is not.
// - At most `TASKS_MAX` output files are followed, the oldest dropped first, and a task's file is
//   forgotten once the task is no longer live. Each keeps `TAIL_LINES` lines, and one read takes at most
//   `TAIL_BYTES`.
// - `stream.ndjson` grows with the session, on disk only. The events log keeps the store's bounds.

import { closeSync, mkdirSync, openSync, readFileSync, readSync, statSync, writeSync } from "node:fs";
import { join } from "node:path";
import type { Config } from "../goal/config.ts";
import { spawnTree, type Tree } from "../os/index.ts";
import { LogWriter } from "../store/segments.ts";
import {
  type AgentAdapter,
  type AgentEvent,
  context,
  kind,
  type LaunchOptions,
  MAX_LINE,
  type RateLimit,
  type ResultEvent,
  SUMMARY_CHARS,
  type SessionEnd,
  summary,
  type Task,
  tool,
  toolArg,
  type Usage,
  written,
} from "./adapter.ts";

/** The file in a session's folder that holds its raw stdout and stderr lines. */
export const STREAM = "stream.ndjson";
/** The most written paths a session records. */
export const WRITTEN_MAX = 10_000;
/** The most notes a session keeps for the next one. */
export const NOTES_MAX = 32;
/** How many controls wait for the session to take them. */
export const CONTROL_BOUND = 64;
/** How long a stdin write may wait for the agent to read before stdin counts as closed, in ms. */
const WRITE_WAIT = 10_000;
/** How often the timers are checked, in ms. */
const TICK = 250;
/** How long the loop sleeps when no pipe had anything, in ms. */
const POLL = 10;
/** The least time between two stats snapshots, in ms. */
export const STATS_EVERY = 1_000;
/** The most background tasks a stats snapshot names. */
export const TASKS_MAX = 32;
/** The most open tool calls the trackers follow at once. */
export const OPEN_MAX = 32;
/** The most lines of a background task's output a snapshot carries. */
export const TAIL_LINES = 3;
/** How much of the end of a task's output file one read takes. */
export const TAIL_BYTES = 4_096;
/** How often the live tasks' output files are checked for growth, in ms. */
const TAIL_EVERY = 2_000;
/**
 * How long the stdin gate stays open for a background task after a `result`, in ms: 2 h 5 min, past the
 * harness's 2-hour ceiling for a background command, so the harness's own limit ends the wait first.
 */
export const BACKGROUND_WAIT = 125 * 60_000;

/** Injected once at the hard budget. */
export const HANDOFF_PROMPT =
  "looper here: your context is past the hand-off budget. Stop taking new work now. Finish or revert the edit you are in the middle of, then hand off: overwrite the live goal's ## Handoff with where the work stands and the next group with anchors, commit what is finished (one commit per slice), write the status line, and stop. Do not run the full verify if it was not already green.";

/** An operator's action on the running session. */
export type Control =
  | { kind: "say"; text: string }
  /** Stop the running turn, and send this prompt as the next one. */
  | { kind: "interrupt"; text: string }
  | { kind: "freeze" }
  | { kind: "thaw" }
  /** Kill the tree now, for this reason. */
  | { kind: "kill"; why: string };

/** The controls waiting for a session, at most `CONTROL_BOUND` of them. */
export class Controls {
  private queue: Control[] = [];

  /** Queues `c` for the session. Refuses it, and says so, while `CONTROL_BOUND` wait. */
  send(c: Control): boolean {
    if (this.queue.length >= CONTROL_BOUND) return false;
    this.queue.push(c);
    return true;
  }

  /** The oldest waiting control, or undefined with none. */
  take(): Control | undefined {
    return this.queue.shift();
  }
}

/** One event as the live view sees it. */
export interface Live {
  seq: number;
  ts: number;
  kind: string;
  summary: string;
  /** The raw line's length in bytes. */
  len: number;
  /** The tool call a call or its result belongs to. */
  tool: string | null;
}

/** The running session's numbers as they stand. */
export interface Stats {
  /** When the session started, in ms since the Unix epoch. */
  started: number;
  /** The context of the newest usage report. */
  context: number;
  peakContext: number;
  /** The hand-off budget and the kill budget the context is held to. Without a kill budget, the session is never killed for its context. */
  hard: number;
  kill: number | null;
  toolCalls: number;
  /** Every input token of each message, cached or not, and every output token. */
  tokensIn: number;
  tokensOut: number;
  /** The first `TASKS_MAX` background tasks of the agent's live set, in its order. */
  tasks: Running[];
  /** How many live tasks are past `TASKS_MAX`. */
  tasksMore: number;
  /** The oldest tool call whose result has not come. */
  openCall: OpenCall | null;
  /** The session's newest tool call, open or ended. */
  lastCall: LastCall | null;
  /** Whether no turn runs, so the agent waits for a background task's report or for stdin to close. */
  idle: boolean;
  /** The goal file's modified time, in ms since the Unix epoch, or null without one. */
  goalStamp: number | null;
}

/** A background task, when the session first saw it, in ms since the Unix epoch, and the last lines of its output. */
export interface Running {
  task: Task;
  since: number;
  /** At most `TAIL_LINES` lines, oldest first, empty while its output file is unknown or empty. */
  tail: string[];
}

/** A tool call whose result has not come: its tool, what it is about, and when it was called, in ms since the Unix epoch. */
export interface OpenCall {
  name: string;
  arg: string;
  since: number;
}

/** A tool call, and when its result came, in ms since the Unix epoch. A turn's `result` ends a call whose result never came. */
export interface LastCall {
  call: OpenCall;
  ended: number | null;
}

/** A live task's output file, how long it was at the last read, and the tail that read found. */
interface Output {
  task: string;
  path: string;
  len: number;
  tail: string[];
}

/** What bounds one session. Every time is in ms. */
export interface Limits {
  hard: number;
  kill: number | null;
  spinLimit: number;
  timeout: number;
  backgroundWait: number;
  followUpGrace: number;
}

/** The limits `cfg` sets. */
export function limitsFrom(cfg: Config): Limits {
  return {
    hard: cfg.budget.hard,
    kill: cfg.budget.kill ?? null,
    spinLimit: cfg.loop.spin_limit,
    timeout: cfg.loop.session_timeout_min * 60_000,
    backgroundWait: BACKGROUND_WAIT,
    followUpGrace: 2 * 60_000,
  };
}

/** Everything one session is launched with. */
export interface Launch {
  adapter: AgentAdapter;
  opts: LaunchOptions;
  /** The first stdin line. */
  opening: string;
  limits: Limits;
  /** The session's folder, which gets `stream.ndjson` and `events/`. */
  dir: string;
  /** Takes each event once it is in the events log. */
  live?: (e: Live) => void;
  /** Takes the session's `Stats` while it runs, and null once it has ended. */
  stats?: (s: Stats | null) => void;
  /** The goal file the session works, whose modified time each snapshot carries. */
  goal?: string;
  controls?: Controls;
}

/** What a session left behind. */
export interface Served {
  /** The exit code, or null when the process ended without one. */
  code: number | null;
  /** The agent's own session id, for `--resume`. */
  agentSession: string;
  result: ResultEvent | null;
  limit: RateLimit | null;
  peakContext: number;
  /** The context of the first usage report: the session's fixed overhead. */
  firstContext: number;
  toolCalls: number;
  /** The paths the session wrote, as the agent named them. */
  written: Set<string>;
  /** Says that arrived while stdin was closed, for the next session. */
  notes: string[];
  /** Paths and notes beyond their bounds. */
  dropped: number;
  /** Why the session was killed, when it was. */
  killed: string | null;
  handoffSent: boolean;
  /** How many events were recorded. */
  events: number;
  truncatedLines: number;
}

/** What the adapter classifies the exit of `s` by, at `now` in unix seconds. */
export function sessionEnd(s: Served, now: number): SessionEnd {
  return { code: s.code, result: s.result, limit: s.limit, now };
}

/** Splits a stream into lines of at most `MAX_LINE` bytes, each without its line ending, and says which were cut. */
export class Lines {
  private parts: Buffer[] = [];
  private kept = 0;
  private cut = false;

  constructor(private readonly max: number = MAX_LINE) {}

  /** The lines `chunk` ends. A partial line stays for the next chunk. */
  push(chunk: Uint8Array): [Buffer, boolean][] {
    const out: [Buffer, boolean][] = [];
    let start = 0;
    for (let end = chunk.indexOf(0x0a); end >= 0; end = chunk.indexOf(0x0a, start)) {
      this.keep(chunk.subarray(start, end));
      out.push(this.take());
      start = end + 1;
    }
    this.keep(chunk.subarray(start));
    return out;
  }

  /** The last line when the stream ended without a newline, or null with none. */
  end(): [Buffer, boolean] | null {
    return this.kept === 0 && !this.cut ? null : this.take();
  }

  private keep(bytes: Uint8Array): void {
    const n = Math.min(bytes.length, this.max - this.kept);
    if (n > 0) this.parts.push(Buffer.from(bytes.subarray(0, n)));
    this.kept += n;
    this.cut ||= n < bytes.length;
  }

  private take(): [Buffer, boolean] {
    let line = Buffer.concat(this.parts, this.kept);
    if (line.at(-1) === 0x0d) line = line.subarray(0, -1);
    const cut = this.cut;
    this.parts = [];
    this.kept = 0;
    this.cut = false;
    return [line, cut];
  }
}

/** Runs one session to its end. */
export async function serve(l: Launch): Promise<Served> {
  const cmd = l.adapter.command(l.opts);
  mkdirSync(l.dir, { recursive: true });
  const tree = spawnTree(cmd.argv, {
    cwd: cmd.cwd,
    env: { ...process.env, ...cmd.env },
    pipe: true,
    split: true,
    input: true,
  });
  const s = new Supervisor(l, tree);
  try {
    return await s.run();
  } finally {
    s.close();
  }
}

class Supervisor {
  private readonly limits: Limits;
  private readonly stream: number;
  private streamLen = 0;
  private readonly events: LogWriter;
  private stdinOpen = true;
  private readonly out: Served = {
    code: null,
    agentSession: "",
    result: null,
    limit: null,
    peakContext: 0,
    firstContext: 0,
    toolCalls: 0,
    written: new Set(),
    notes: [],
    dropped: 0,
    killed: null,
    handoffSent: false,
    events: 0,
    truncatedLines: 0,
  };
  private turnRunning = true;
  private tasksLive = 0;
  /** The first `TASKS_MAX` of the live tasks. */
  private tasks: Running[] = [];
  /** At most `OPEN_MAX` open tool calls, by id, oldest first. */
  private open: [string, OpenCall][] = [];
  /** The newest tool call, by id. */
  private lastCall: [string, LastCall] | null = null;
  /** The output files of at most `TASKS_MAX` tasks, oldest first, and when they were last checked. */
  private outputs: Output[] = [];
  private outputsAt: number | null = null;
  private waitingSince: number | null = null;
  private idleSince: number | null = null;
  /** The hash of the last tool call and how many times in a row it came. */
  private spin: [bigint | number, number] = [0, 0];
  /** How many interrupts were sent, which numbers the next one. */
  private interrupts = 0;
  /** Whether an interrupt was sent and the stopped turn's result has not arrived. */
  private interrupting = false;
  private readonly started = Date.now();
  /** The context of the newest usage report. */
  private context = 0;
  private tokens: [number, number] = [0, 0];
  /** The newest usage report, so a message split into several lines is counted once. */
  private lastUsage: Usage | null = null;
  /** Whether the stats changed since the last snapshot, and when that went out. */
  private statsDue = true;
  private statsAt: number | null = null;

  constructor(
    private readonly l: Launch,
    private readonly tree: Tree,
  ) {
    this.limits = l.limits;
    this.stream = openSync(join(l.dir, STREAM), "a");
    this.streamLen = statSync(join(l.dir, STREAM)).size;
    this.events = new LogWriter(join(l.dir, "events"));
  }

  async run(): Promise<Served> {
    await this.write(Buffer.from(this.l.opening));
    const stdout = new Lines();
    const stderr = new Lines();
    let outOpen = true;
    let errOpen = true;
    let exited = false;
    const deadline = this.started + this.limits.timeout;
    let tickAt = 0;
    while (outOpen || errOpen) {
      let busy = false;
      if (outOpen) {
        const chunk = this.tree.read();
        if (chunk === null) {
          outOpen = false;
          const last = stdout.end();
          if (last) await this.take(...last);
        } else if (chunk.length > 0) {
          busy = true;
          for (const [line, cut] of stdout.push(chunk)) await this.take(line, cut);
        }
      }
      if (errOpen) {
        const chunk = this.tree.readErr();
        if (chunk === null) {
          errOpen = false;
          const last = stderr.end();
          if (last) this.takeStderr(...last);
        } else if (chunk.length > 0) {
          busy = true;
          for (const [line, cut] of stderr.push(chunk)) this.takeStderr(line, cut);
        }
      }
      if (!exited && this.tree.exitCode() !== null) {
        exited = true;
        // Whatever the agent left running would hold its pipes open.
        this.killTree();
      }
      for (let c = this.l.controls?.take(); c !== undefined; c = this.l.controls?.take()) await this.control(c);
      if (this.out.killed === null && Date.now() >= deadline) {
        this.kill(`ran past the ${Math.floor(this.limits.timeout / 60_000)}-minute session timeout`);
      }
      if (Date.now() - tickAt >= TICK) {
        tickAt = Date.now();
        this.tick();
      }
      await Bun.sleep(busy ? 0 : POLL);
    }
    this.closeStdin();
    this.out.code = await this.tree.exited();
    if (this.l.opts.written) this.readWritten(this.l.opts.written);
    return this.out;
  }

  /** Closes the session's files and handles, and tells the stats reader the session has ended. */
  close(): void {
    this.tree.close();
    closeSync(this.stream);
    this.events.close();
    this.l.stats?.(null);
  }

  private appendRaw(line: Uint8Array): [number, number] {
    const offset = this.streamLen;
    writeSync(this.stream, Buffer.concat([line, Buffer.from("\n")]));
    this.streamLen += line.length + 1;
    return [offset, line.length];
  }

  private async take(line: Buffer, cut: boolean): Promise<void> {
    if (line.toString("latin1").trim() === "") return;
    const [offset, len] = this.appendRaw(line);
    let events: AgentEvent[];
    if (cut) {
      this.out.truncatedLines++;
      events = [{ kind: "other", other: "truncated" }];
    } else events = this.l.adapter.parse(line);
    for (const e of events) {
      this.record(kind(e), summary(e), tool(e), offset, len);
      await this.track(e);
    }
  }

  private takeStderr(line: Buffer, cut: boolean): void {
    const text = line.toString("utf8");
    if (text.trim() === "") return;
    const wrapped = Buffer.from(JSON.stringify({ type: "looper_stderr", text, truncated: cut }));
    const [offset, len] = this.appendRaw(wrapped);
    this.record("stderr", Array.from(text.trim()).slice(0, SUMMARY_CHARS).join(""), null, offset, len);
  }

  private record(kind: string, summary: string, tool: string | null, offset: number, len: number): void {
    this.out.events++;
    const ts = Date.now();
    const seq = this.events.append({ ts, kind, summary, tool, offset, len });
    this.l.live?.({ seq, ts, kind, summary, len, tool });
  }

  private async track(e: AgentEvent): Promise<void> {
    switch (e.kind) {
      case "init":
        if (this.out.agentSession === "") this.out.agentSession = e.sessionId;
        this.busy();
        break;
      case "usage": {
        const u = e.usage;
        this.busy();
        if (!sameUsage(this.lastUsage, u)) {
          this.tokens[0] += u.input + u.cacheRead + u.cacheWrite;
          this.tokens[1] += u.output;
        }
        this.lastUsage = u;
        this.statsDue = true;
        const ctx = context(u);
        this.context = ctx;
        if (this.out.firstContext === 0) this.out.firstContext = ctx;
        this.out.peakContext = Math.max(this.out.peakContext, ctx);
        if (this.limits.kill !== null && ctx >= this.limits.kill) {
          this.kill(`context ${Math.floor(ctx / 1000)}k is past the kill budget`);
        } else if (ctx >= this.limits.hard && !this.out.handoffSent) {
          this.out.handoffSent = await this.inject(HANDOFF_PROMPT);
        }
        break;
      }
      case "text":
        this.busy();
        break;
      case "tool_use": {
        this.busy();
        this.out.toolCalls++;
        this.statsDue = true;
        this.openCall(e.id, e.name, e);
        const path = written(e);
        if (path !== null) this.noteWritten(path);
        const key = Bun.hash(`${e.name}\0${JSON.stringify(e.input ?? null)}`);
        this.spin = this.spin[0] === key ? [key, this.spin[1] + 1] : [key, 1];
        if (this.spin[1] >= this.limits.spinLimit) this.kill(`the same ${e.name} call ${this.spin[1]} times in a row`);
        break;
      }
      case "background":
        this.background(e.tasks);
        if (this.tasksLive === 0 && !this.turnRunning) this.idleSince = Date.now();
        break;
      case "rate_limit":
        this.out.limit = e.limit;
        break;
      case "result":
        this.out.result = e.result;
        this.turnRunning = false;
        this.statsDue = true;
        // A turn that ended leaves no call open, even one whose result never came.
        this.open = [];
        this.endCall(null);
        if (this.interrupting) {
          this.interrupting = false;
          // The stopped turn's result: the prompt behind the interrupt is the next turn, and the
          // follow-up grace closes stdin if it never starts.
          this.waitingSince ??= Date.now();
          if (this.tasksLive === 0) this.idleSince = Date.now();
        } else if (this.tasksLive === 0) this.closeStdin();
        else this.waitingSince ??= Date.now();
        break;
      case "tool_result": {
        const at = this.open.findIndex(([id]) => id === e.id);
        if (at >= 0) {
          this.open.splice(at, 1);
          this.statsDue = true;
        }
        this.endCall(e.id);
        if (e.output) this.output(e.output.task, e.output.path);
        break;
      }
      case "other":
      case "unparsed":
        break;
    }
  }

  /** Makes the tool call `call` the newest, and follows it as open until its result comes, while fewer than `OPEN_MAX` are. */
  private openCall(id: string, name: string, call: AgentEvent): void {
    const open: OpenCall = { name, arg: toolArg(call) ?? "", since: Date.now() };
    this.lastCall = [id, { call: { ...open }, ended: null }];
    if (id !== "" && this.open.length < OPEN_MAX) this.open.push([id, open]);
  }

  /** Ends the newest tool call now, when it is the call `id` or, for null, whichever it is. */
  private endCall(id: string | null): void {
    const last = this.lastCall;
    if (last && last[1].ended === null && (id === null || id === last[0])) {
      last[1].ended = Date.now();
      this.statsDue = true;
    }
  }

  /** Remembers the output file of the task `task`, dropping the oldest past `TASKS_MAX`. The next tick reads its tail. */
  private output(task: string, path: string): void {
    this.outputs = this.outputs.filter((o) => o.task !== task);
    if (this.outputs.length >= TASKS_MAX) this.outputs.shift();
    this.outputs.push({ task, path, len: 0, tail: [] });
    this.outputsAt = null;
  }

  /** Takes the agent's new set of live tasks. A task already live keeps the time it was first seen, and the output file of a task no longer live is forgotten. */
  private background(tasks: Task[]): void {
    this.tasksLive = tasks.length;
    this.outputs = this.outputs.filter((o) => tasks.some((t) => t.id === o.task));
    const now = Date.now();
    const seen = this.tasks;
    this.tasks = tasks.slice(0, TASKS_MAX).map((task) => ({
      task,
      since: seen.find((r) => r.task.id === task.id)?.since ?? now,
      tail: [],
    }));
    this.statsDue = true;
  }

  /** Reads again the tail of each output file whose length changed since the last read. */
  private readOutputs(): void {
    for (const o of this.outputs) {
      let len = 0;
      try {
        len = statSync(o.path).size;
      } catch {}
      if (len !== o.len) {
        o.len = len;
        o.tail = tail(o.path, len);
        this.statsDue = true;
      }
    }
    this.outputsAt = Date.now();
  }

  private busy(): void {
    this.statsDue ||= !this.turnRunning;
    this.turnRunning = true;
    this.idleSince = null;
  }

  private async control(c: Control): Promise<void> {
    switch (c.kind) {
      case "say":
        if (!(await this.inject(c.text))) this.note(c.text);
        break;
      case "interrupt": {
        let sent: boolean;
        if (this.turnRunning) {
          this.interrupts++;
          this.interrupting = await this.write(Buffer.from(this.l.adapter.interrupt(this.interrupts, c.text)));
          sent = this.interrupting;
        } else sent = await this.inject(c.text);
        if (!sent) this.note(c.text);
        break;
      }
      case "freeze":
        this.tree.freeze();
        break;
      case "thaw":
        this.tree.thaw();
        break;
      case "kill":
        this.kill(c.why);
        break;
    }
  }

  private tick(): void {
    if (this.stdinOpen && !this.turnRunning && this.waitingSince !== null) {
      const now = Date.now();
      const waited = now - this.waitingSince >= this.limits.backgroundWait;
      const idle = this.idleSince !== null && now - this.idleSince >= this.limits.followUpGrace;
      if (waited || idle) this.closeStdin();
    }
    if (this.outputs.length > 0 && (this.outputsAt === null || Date.now() - this.outputsAt >= TAIL_EVERY)) {
      this.readOutputs();
    }
    if (this.statsDue && (this.statsAt === null || Date.now() - this.statsAt >= STATS_EVERY)) this.sendStats();
  }

  private sendStats(): void {
    if (!this.l.stats) return;
    this.l.stats({
      started: this.started,
      context: this.context,
      peakContext: this.out.peakContext,
      hard: this.limits.hard,
      kill: this.limits.kill,
      toolCalls: this.out.toolCalls,
      tokensIn: this.tokens[0],
      tokensOut: this.tokens[1],
      tasks: this.tasks.map((r) => ({
        ...r,
        tail: [...(this.outputs.find((o) => o.task === r.task.id)?.tail ?? [])],
      })),
      tasksMore: Math.max(0, this.tasksLive - this.tasks.length),
      openCall: this.open[0] ? { ...this.open[0][1] } : null,
      lastCall: this.lastCall ? { call: { ...this.lastCall[1].call }, ended: this.lastCall[1].ended } : null,
      idle: !this.turnRunning,
      goalStamp: this.l.goal ? stamp(this.l.goal) : null,
    });
    this.statsDue = false;
    this.statsAt = Date.now();
  }

  private kill(why: string): void {
    if (this.out.killed !== null) return;
    this.out.killed = why;
    this.closeStdin();
    this.killTree();
  }

  private killTree(): void {
    try {
      this.tree.kill();
    } catch {
      // A job whose processes have all ended has nothing left to kill.
    }
  }

  private closeStdin(): void {
    this.stdinOpen = false;
    this.tree.closeInput();
  }

  /** Writes a user line through the adapter. Says whether it reached an open stdin. */
  private inject(text: string): Promise<boolean> {
    return this.write(Buffer.from(this.l.adapter.inject(text)));
  }

  /** Writes `bytes` to stdin within `WRITE_WAIT`. Says whether all of it went, and closes stdin when not. */
  private async write(bytes: Buffer): Promise<boolean> {
    if (!this.stdinOpen) return false;
    const end = Date.now() + WRITE_WAIT;
    let at = 0;
    while (at < bytes.length) {
      const put = this.tree.write(bytes.subarray(at));
      if (put === null || Date.now() >= end) {
        this.closeStdin();
        return false;
      }
      at += put;
      if (at < bytes.length) await Bun.sleep(POLL);
    }
    return true;
  }

  /** Keeps `text` for the next session's pack. */
  private note(text: string): void {
    if (this.out.notes.length < NOTES_MAX) this.out.notes.push(text);
    else this.out.dropped++;
  }

  private noteWritten(path: string): void {
    if (this.out.written.size < WRITTEN_MAX || this.out.written.has(path)) this.out.written.add(path);
    else this.out.dropped++;
  }

  /** Adds the paths tools reported through `LOOPER_WRITTEN`. */
  private readWritten(list: string): void {
    let text: string;
    try {
      text = readFileSync(list, "utf8");
    } catch {
      return;
    }
    for (const line of text.split("\n").slice(0, WRITTEN_MAX)) {
      if (line.trim() !== "") this.noteWritten(line.trim());
    }
  }
}

function sameUsage(a: Usage | null, b: Usage): boolean {
  return (
    a !== null && a.input === b.input && a.output === b.output && a.cacheRead === b.cacheRead && a.cacheWrite === b.cacheWrite
  );
}

/** The modified time of the file at `path`, in ms since the Unix epoch, or null when it cannot be read. */
function stamp(path: string): number | null {
  try {
    return Math.floor(statSync(path).mtimeMs);
  } catch {
    return null;
  }
}

/**
 * The last `TAIL_LINES` lines with text of the file at `path`, which is `len` bytes long, read from its
 * last `TAIL_BYTES` bytes, oldest first. A line a carriage return rewrote keeps its last part, and each is
 * cut to `SUMMARY_CHARS` characters without terminal escapes. Empty when the file cannot be read.
 */
export function tail(path: string, len: number): string[] {
  const from = Math.max(0, len - TAIL_BYTES);
  const buf = Buffer.alloc(Math.min(TAIL_BYTES, len));
  let got: number;
  try {
    const fd = openSync(path, "r");
    try {
      got = readSync(fd, buf, 0, buf.length, from);
    } finally {
      closeSync(fd);
    }
  } catch {
    return [];
  }
  // A read from the middle of the file starts inside a line.
  const whole = buf.subarray(0, got).toString("utf8").split("\n").slice(from > 0 ? 1 : 0);
  const lines = whole
    .map((l) => l.split("\r").reverse().find((part) => part.trim() !== ""))
    .filter((l): l is string => l !== undefined)
    .map(plain)
    .filter((l) => l.trim() !== "");
  return lines.slice(Math.max(0, lines.length - TAIL_LINES));
}

/** `line` without terminal escape sequences and control characters, cut to `SUMMARY_CHARS` characters. */
function plain(line: string): string {
  let out = "";
  const chars = Array.from(line.trimEnd());
  for (let i = 0; i < chars.length; i++) {
    const c = chars[i] as string;
    if (c === "\x1b") {
      // A CSI sequence ends at its first letter. Any other escape drops the one character after it.
      if (chars[++i] === "[") while (++i < chars.length && !/[A-Za-z]/.test(chars[i] as string));
    } else if (c === "\t" || !/\p{Cc}/u.test(c)) out += c;
  }
  return Array.from(out).slice(0, SUMMARY_CHARS).join("");
}
