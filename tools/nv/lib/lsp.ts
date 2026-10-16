// The language server's base protocol, for a tool that sends `nvs lsp` a whole session on stdin: a
// message framed the way the server reads it, and the messages it wrote, read back from its standard
// output once it has exited.

/** One message, framed the way `nvs lsp` reads it from stdin. */
export const lspFrame = (message: object): string => {
  const body = JSON.stringify({ jsonrpc: "2.0", ...message });
  return `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`;
};

/** Every message in `out`, the whole standard output of an `nvs lsp` that has exited, in the order it
 * was written. A frame cut short at the end is not a message. */
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
