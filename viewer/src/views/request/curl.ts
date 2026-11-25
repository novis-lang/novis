// *Copy as curl* on a request's page: a `curl` command built from the recorded method, path, query,
// headers and body. A header the server redacted keeps the value `[redacted]`, as a test made from the
// request does. The address is the recorded `Host` header, or `localhost` without one.
//
// Each argument is in single quotes for a POSIX shell, so nothing in a recorded value is run.

import type { RecordedInput, RequestRow } from "../../api.ts";

/** Headers curl writes itself from the command, which a copy of them would only duplicate. */
const OWN = new Set(["host", "content-length", "connection", "transfer-encoding"]);

/** `value` in single quotes, with each `'` closed, escaped and opened again. */
export function quote(value: string): string {
  return `'${value.replaceAll("'", `'\\''`)}'`;
}

/** The command, and whether the body could not be included because it is binary, cut, withheld or
 *  was not read. */
export function curl(row: Pick<RequestRow, "method" | "path">, input: RecordedInput): { command: string; bodyLeftOut: boolean } {
  const host = input.headers.find(([name]) => name.toLowerCase() === "host")?.[1] ?? "localhost";
  const url = `http://${host}${row.path}${input.query ? `?${input.query}` : ""}`;
  const parts = ["curl"];
  if (row.method !== "GET") parts.push("-X", quote(row.method));
  parts.push(quote(url));
  for (const [name, value] of input.headers) {
    if (!OWN.has(name.toLowerCase())) parts.push("-H", quote(`${name}: ${value}`));
  }
  const body = input.body;
  const usable = body.read && !body.cut && !body.withheld && body.text !== null;
  if (usable && body.text !== "") parts.push("--data-raw", quote(body.text!));
  const hasBytes = body.text !== "" && (body.text !== null || body.base64 !== null);
  return { command: parts.join(" "), bodyLeftOut: !usable && (hasBytes || body.withheld || body.cut) };
}
