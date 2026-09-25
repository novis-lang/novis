// A session's stream-json events on the console, the way `tools/loop.py` showed them and the way Claude
// Code's own transcript reads: the assistant's text in white, its thinking in magenta behind `. `, every
// tool call in cyan behind `> ` with all of its input under it, and each result behind `< ` in green, or
// behind `! ` in red when the call failed. A result or thinking block longer than `maxResultLines`, an
// input longer than `maxInputLines` and a line longer than `maxLineChars` are cut, and the cut says how
// much it hid; 0 removes a cap. The session's log holds every event whole either way.
//
// It counts the session's context tokens for the line the driver prints when the session ends. The status
// line reads the same events through `driver/status.ts`'s `Session`.

import { C, ktok, say } from "./console.ts";

export interface Caps {
  maxResultLines: number;
  maxInputLines: number;
  maxLineChars: number;
}

export const DEFAULT_CAPS: Caps = { maxResultLines: 60, maxInputLines: 40, maxLineChars: 500 };

export class Renderer {
  private names = new Map<string, string>();
  /** The last assistant text printed: the `result` event repeats it, and is not printed twice. */
  private said = "";
  /** The context of the session's latest turn, in tokens. */
  private context = 0;
  /** Output tokens per assistant message id: a message arrives once per content block, each repeating its usage. */
  private written = new Map<string, number>();

  constructor(
    private readonly caps: Caps,
    /** The `init` event carries no effort, so the one the driver passed is shown. */
    private readonly effort = "",
  ) {}

  /** A session is starting, and its token counts start with it. */
  begin(): void {
    this.context = 0;
    this.written.clear();
    this.said = "";
  }

  /** `ctx 84.2k in / 6.1k out`, or "" before the first turn reports. */
  tokens(): string {
    if (!this.context) return "";
    let out = 0;
    for (const n of this.written.values()) out += n;
    return `ctx ${ktok(this.context)} in / ${ktok(out)} out`;
  }

  /** In is the latest turn's whole window; out is summed over the session's messages. A subagent's turns are skipped. */
  private count(e: Record<string, any>): void {
    if (e.parent_tool_use_id) return;
    const message = e.message ?? {};
    const usage = message.usage ?? {};
    const context = ["input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"].reduce((n, k) => n + Number(usage[k] ?? 0), 0);
    if (context) this.context = context;
    if (usage.output_tokens) this.written.set(String(message.id ?? this.written.size), Number(usage.output_tokens));
  }

  private wrapped(text: unknown, prefix: string, colour: string, maxLines: number): void {
    if (text === undefined || text === null || text === "") return;
    const body = String(text).replace(/\t/g, "    ").trimEnd();
    if (!body) return;
    let lines = body.split("\n");
    let hidden = 0;
    if (maxLines > 0 && lines.length > maxLines) {
      hidden = lines.length - maxLines;
      lines = lines.slice(0, maxLines);
    }
    const max = this.caps.maxLineChars;
    for (let line of lines) {
      line = line.replace(/\r$/, "");
      if (max > 0 && line.length > max) line = `${line.slice(0, max)}  [+${line.length - max} chars]`;
      say(prefix + line, colour);
    }
    if (hidden) say(`${prefix}... ${hidden} more line(s) -- full text in the session log`, C.GRAY);
  }

  /** A message's `content`: a plain string or a list of blocks. */
  private static contentText(content: unknown): string {
    if (content === undefined || content === null) return "";
    if (typeof content === "string") return content;
    const parts: string[] = [];
    for (const block of Array.isArray(content) ? content : [content]) {
      if (typeof block === "string") parts.push(block);
      else if (block && typeof block === "object") {
        if (block.text) parts.push(String(block.text));
        else if (block.type === "image") parts.push("[image]");
        else parts.push(JSON.stringify(block));
      }
    }
    return parts.join("\n");
  }

  /** Every argument of a tool call. */
  private toolInput(input: unknown): void {
    if (!input || typeof input !== "object") return;
    for (const [name, value] of Object.entries(input)) {
      if (value === undefined || value === null) continue;
      const text = typeof value === "string" ? value : JSON.stringify(value, null, 2);
      if (!text.trim()) continue;
      if (text.includes("\n")) {
        say(`       ${name}:`, C.DIM_CYAN);
        this.wrapped(text, "       | ", C.DIM_CYAN, this.caps.maxInputLines);
      } else {
        this.wrapped(`${name}: ${text}`, "       ", C.DIM_CYAN, 1);
      }
    }
  }

  /** One event off the session's stdout. */
  event(e: Record<string, any>): void {
    const kind = e.type;
    if (kind === "system") {
      if (e.subtype === "init") {
        say(`   [init] claude ${e.claude_code_version || "?"}, model ${e.model || "?"}, effort ${this.effort || "model default"}`, C.CYAN);
        say(`   [init] cwd=${e.cwd} session=${e.session_id}`, C.GRAY);
        if (Array.isArray(e.tools) && e.tools.length) this.wrapped(`tools: ${e.tools.join(", ")}`, "   [init] ", C.GRAY, 2);
      } else {
        this.wrapped(JSON.stringify(e), `   [${e.subtype}] `, C.GRAY, 4);
      }
    } else if (kind === "assistant") {
      this.count(e);
      for (const b of e.message?.content ?? []) {
        if (b.type === "text") {
          this.said = String(b.text ?? "");
          this.wrapped(b.text, "   ", C.WHITE, 0);
        } else if (b.type === "thinking") {
          this.wrapped(b.thinking, "   . ", C.MAGENTA, this.caps.maxResultLines);
        } else if (b.type === "tool_use") {
          if (b.id) this.names.set(String(b.id), String(b.name));
          say(`   > ${b.name}`, C.CYAN);
          this.toolInput(b.input);
        }
      }
    } else if (kind === "user") {
      const content = e.message?.content;
      for (const b of Array.isArray(content) ? content : []) {
        if (!b || typeof b !== "object") continue;
        if (b.type === "tool_result") {
          const name = this.names.get(String(b.tool_use_id)) ?? "result";
          const text = Renderer.contentText(b.content);
          if (b.is_error) {
            say(`     ! ${name} failed`, C.RED);
            this.wrapped(text, "     | ", C.RED, this.caps.maxResultLines);
          } else {
            say(`     < ${name}`, C.DIM_GREEN);
            this.wrapped(text, "     | ", C.GRAY, this.caps.maxResultLines);
          }
        } else if (b.type === "text") {
          this.wrapped(b.text, "   + ", C.WHITE, this.caps.maxResultLines);
        }
      }
    } else if (kind === "result") {
      const bits = [`${e.num_turns} turns`];
      if (e.duration_ms !== undefined && e.duration_ms !== null) bits.push(`${(Number(e.duration_ms) / 1000).toFixed(1)}s`);
      const usage = e.usage ?? {};
      if (Object.keys(usage).length) bits.push(`in ${usage.input_tokens} / out ${usage.output_tokens} tok`);
      if (e.total_cost_usd !== undefined && e.total_cost_usd !== null) bits.push(`$${Number(e.total_cost_usd).toFixed(2)}`);
      say(`   [${e.subtype}] ${bits.join("  ")}`, C.YELLOW);
      // The final message is already printed from its assistant event; the result repeats it only for an ending no assistant event carried.
      const body = String(e.result ?? "");
      if (body.trim() && body.trim() !== this.said.trim()) this.wrapped(body, "   ", C.YELLOW, this.caps.maxResultLines);
    }
  }
}
