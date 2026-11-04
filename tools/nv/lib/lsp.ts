// The language server's base protocol, for a tool that talks to `nvs lsp` over its stdin: a message
// framed the way the server reads it, a session written a part at a time as an editor writes one
// (`lspTalk`), and the messages the server wrote, read back from its standard output.

import type { RunOptions } from "./proc.ts";

/** One message, framed the way `nvs lsp` reads it from stdin. */
export const lspFrame = (message: object): string => {
  const body = JSON.stringify({ jsonrpc: "2.0", ...message });
  return `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`;
};

/** One part of a session: framed messages that end with the request `answer` names, or with no request
 * when `answer` is null. */
export interface LspPart {
  frames: string;
  answer: number | null;
}

/** `messages` cut into parts, each ending after one request, so a request is the last thing the server
 * reads before it answers. */
export function lspParts(messages: object[]): LspPart[] {
  const parts: LspPart[] = [];
  let frames = "";
  for (const m of messages) {
    frames += lspFrame(m);
    if ("id" in m) {
      parts.push({ frames, answer: m.id as number });
      frames = "";
    }
  }
  if (frames !== "") parts.push({ frames, answer: null });
  return parts;
}

/**
 * The `talk` and `onText` of a `proc.run` that writes `parts` to the server: each part once the server
 * has answered the request that ends the part before it, and then the end of stdin. An editor waits the
 * same way, since a person types the next key after the list is shown. A part written before the answer
 * would reach the server while it still answers, and the server answers a request whose document has
 * a later change queued with `ContentModified`.
 */
export function lspTalk(parts: LspPart[]): Pick<RunOptions, "talk" | "onText"> {
  let out = "";
  let next = 0;
  let stdin: { write(text: string): void; end(): void } | null = null;
  const answered = (id: number) => lspMessages(out).some((m) => m.id === id && !("method" in m));
  const pump = () => {
    if (stdin === null) return;
    while (next < parts.length) {
      const before = next === 0 ? null : parts[next - 1]!.answer;
      if (before !== null && !answered(before)) return;
      stdin.write(parts[next]!.frames);
      next++;
    }
    stdin.end();
  };
  return {
    talk: (s) => {
      stdin = s;
      pump();
    },
    onText: (text, stream) => {
      if (stream !== "stdout") return;
      out += text;
      pump();
    },
  };
}

/** Every message in `out`, the standard output of an `nvs lsp` so far, in the order it was written. A
 * frame cut short at the end is not a message. */
export function lspMessages(out: string): Record<string, unknown>[] {
  let bytes = Buffer.from(out, "utf8");
  const got: Record<string, unknown>[] = [];
  for (;;) {
    const end = bytes.indexOf("\r\n\r\n");
    if (end < 0) return got;
    const length = Number(/Content-Length:\s*(\d+)/i.exec(bytes.subarray(0, end).toString())?.[1]);
    if (!Number.isFinite(length) || bytes.length < end + 4 + length) return got;
    got.push(JSON.parse(bytes.subarray(end + 4, end + 4 + length).toString("utf8")));
    bytes = bytes.subarray(end + 4 + length);
  }
}
