// A `.dap`'s session with `nvs dap`, and the `.dap.out` it records. `tools/nv/cmd/site.ts` § *A snippet*
// is the home of both formats; this module reads the first, holds the session its lines describe, and
// writes what the adapter returned in the second.
//
// The session is a conversation, where the `.replay` one is a stream: the adapter answers `stackTrace`
// only while the program is stopped, so each stop's requests are sent once its `stopped` event has
// arrived. `recordDap` sends `initialize`, the breakpoints, `launch` and `configurationDone`, and at each
// stop the threads, the stack, the first frame's scopes and the first scope's variables; then the next
// step, until the steps run out or the program ends, and then `disconnect`.
//
// A number that only names an object for one session, such as a frame id or a variables reference, is
// never written, so two runs of the same session record the same text.

import { basename, join } from "node:path";
import { ROOT } from "./paths.ts";
import type { RunOptions, RunResult } from "./proc.ts";

/** A parsed `.dap`: the breakpoint lines, counted from 1, and the steps. */
export interface Dap {
  breaks: number[];
  steps: ("continue" | "next" | "in" | "out")[];
}

/** The `.dap` parser the page uses too: `website/config/replay-dap.mjs`, imported by path so `tsc` reads no
 * `.mjs` of the site's. `parseDap` returns a `Dap`, or why the text is not one; `DAP_STEPS` is the request
 * each step sends. */
const dap = (await import(join(ROOT, "website/config/replay-dap.mjs"))) as {
  DAP_STEPS: Record<Dap["steps"][number], string>;
  parseDap(text: string): Dap | string;
};
export const parseDap = dap.parseDap;

type Message = Record<string, any>;

/** Starts `nvs dap` with `talk` and `onText` as `proc.run` takes them, and gives the run once it ends. */
export type StartDap = (opts: Pick<RunOptions, "talk" | "onText">) => Promise<RunResult>;

/** The messages the adapter has written, read from its standard output as it arrives, and a wait for the
 * first one that fits. A message no wait took stays, so an event that arrives early is not lost. */
class Inbox {
  private bytes = Buffer.alloc(0);
  private messages: Message[] = [];
  private waits: { fits: (m: Message) => boolean; done: (m: Message | null) => void }[] = [];
  private ended = false;

  add(text: string): void {
    this.bytes = Buffer.concat([this.bytes, Buffer.from(text, "utf8")]);
    for (;;) {
      const end = this.bytes.indexOf("\r\n\r\n");
      if (end < 0) break;
      const length = Number(/Content-Length:\s*(\d+)/i.exec(this.bytes.subarray(0, end).toString())?.[1]);
      if (!Number.isFinite(length) || this.bytes.length < end + 4 + length) break;
      this.messages.push(JSON.parse(this.bytes.subarray(end + 4, end + 4 + length).toString("utf8")));
      this.bytes = this.bytes.subarray(end + 4 + length);
    }
    this.settle();
  }

  /** The adapter has ended, so every wait still open gets null. */
  end(): void {
    this.ended = true;
    this.settle();
  }

  /** The first message that fits, taken out of the inbox; null once the adapter ends without one. */
  wait(fits: (m: Message) => boolean): Promise<Message | null> {
    return new Promise((done) => {
      this.waits.push({ fits, done });
      this.settle();
    });
  }

  private settle(): void {
    this.waits = this.waits.filter((w) => {
      const i = this.messages.findIndex(w.fits);
      if (i >= 0) w.done(this.messages.splice(i, 1)[0]!);
      else if (this.ended) w.done(null);
      return i < 0 && !this.ended;
    });
  }
}

/** A value as one line of a `.dap.out`: a line break is written `\n` and a tab `\t`. */
const oneLine = (s: unknown) => String(s ?? "").replace(/\r?\n/g, "\\n").replace(/\t/g, "\\t");

/** The `.dap.out` text `dap` records for `program`, an absolute path, from the session `start` holds with
 * `nvs dap`; or why there is none. */
export async function recordDap(program: string, steps: Dap, start: StartDap): Promise<{ recording: string } | { failure: string }> {
  const inbox = new Inbox();
  let write: (text: string) => void = () => {};
  let close: () => void = () => {};
  const ran = start({
    talk: (stdin) => {
      write = stdin.write;
      close = stdin.end;
    },
    onText: (text, stream) => {
      if (stream === "stdout") inbox.add(text);
    },
  });
  void ran.finally(() => inbox.end());
  const failed = async (why: string) => {
    close();
    const done = await ran;
    return { failure: `${why} (exit ${done.code}: ${done.stderr.trim().split("\n")[0] ?? ""})` };
  };
  let seq = 0;
  const send = (command: string, args: object) => {
    const body = JSON.stringify({ seq: ++seq, type: "request", command, arguments: args });
    write(`Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`);
    return seq;
  };
  /** The body of the response to `command`, or why there is none. */
  const ask = async (command: string, args: object): Promise<Message | string> => {
    const id = send(command, args);
    const got = await inbox.wait((m) => m.type === "response" && m.request_seq === id);
    if (got === null) return `\`nvs dap\` ended before it answered \`${command}\``;
    if (!got.success) return `\`${command}\` failed: ${got.message ?? "no message"}`;
    return got.body ?? {};
  };

  const blocks: string[] = [];
  /** The thread that stopped last, which each step names. */
  let current: unknown = undefined;
  /** Waits for the program to stop or end, and records the block for `step`; true once it has ended. */
  const settle = async (step: number): Promise<boolean | string> => {
    const event = await inbox.wait((m) => m.type === "event" && (m.event === "stopped" || m.event === "exited"));
    if (event === null) return `\`nvs dap\` ended before the program stopped or ended`;
    if (event.event === "exited") {
      blocks.push(`end ${step} ${event.body?.exitCode ?? 0}`);
      return true;
    }
    const threads = await ask("threads", {});
    if (typeof threads === "string") return threads;
    const threadId = event.body?.threadId ?? threads.threads?.[0]?.id;
    const trace = await ask("stackTrace", { threadId });
    if (typeof trace === "string") return trace;
    const frames: Message[] = trace.stackFrames ?? [];
    const lines = [`stop ${step} ${event.body?.reason ?? "unknown"}`];
    for (const f of frames) {
      const file = f.source?.name ?? (f.source?.path ? basename(f.source.path) : "-");
      lines.push(`  frame ${oneLine(file)}:${f.line ?? 0} ${oneLine(f.name)}`);
    }
    if (frames[0]) {
      const scopes = await ask("scopes", { frameId: frames[0].id });
      if (typeof scopes === "string") return scopes;
      const reference = scopes.scopes?.[0]?.variablesReference ?? 0;
      if (reference !== 0) {
        const variables = await ask("variables", { variablesReference: reference });
        if (typeof variables === "string") return variables;
        for (const v of variables.variables ?? []) lines.push(`  var ${oneLine(v.name)}\t${oneLine(v.type)}\t${oneLine(v.value)}`);
      }
    }
    blocks.push(lines.join("\n"));
    current = threadId;
    return false;
  };

  const init = await ask("initialize", { adapterID: "novis", linesStartAt1: true, columnsStartAt1: true, supportsVariableType: true });
  if (typeof init === "string") return failed(init);
  if ((await inbox.wait((m) => m.type === "event" && m.event === "initialized")) === null) return failed("`nvs dap` sent no `initialized` event");
  const set = await ask("setBreakpoints", { source: { path: program }, breakpoints: steps.breaks.map((line) => ({ line })) });
  if (typeof set === "string") return failed(set);
  send("launch", { mode: "run", program });
  const configured = await ask("configurationDone", {});
  if (typeof configured === "string") return failed(configured);
  let ended = await settle(0);
  if (typeof ended === "string") return failed(`the start: ${ended}`);
  for (const [i, step] of steps.steps.entries()) {
    if (ended) return failed(`step ${i + 1}: \`${step}\` after the program ended`);
    const sent = await ask(dap.DAP_STEPS[step], { threadId: current });
    if (typeof sent === "string") return failed(`step ${i + 1}: ${sent}`);
    ended = await settle(i + 1);
    if (typeof ended === "string") return failed(`step ${i + 1}: ${ended}`);
  }
  await ask("disconnect", { terminateDebuggee: true });
  close();
  const done = await ran;
  if (done.timedOut) return { failure: `\`nvs dap\` did not end after \`disconnect\`${done.hung ? " (it hung)" : ""}` };
  return { recording: `${blocks.join("\n\n")}\n` };
}
