import { describe, expect, test } from "bun:test";
import type { RecordedBody, RecordedInput } from "../src/api.ts";
import { curl, quote } from "../src/views/request/curl.ts";

// *Copy as curl* builds a shell command from recorded text, so every recorded value must reach the
// shell as one quoted argument that runs nothing.

const body = (over: Partial<RecordedBody> = {}): RecordedBody => ({ text: "", base64: null, read: true, cut: false, withheld: false, ...over });
const input = (over: Partial<RecordedInput> = {}): RecordedInput => ({
  query: "",
  headers: [],
  body: body(),
  response_headers: null,
  response_body: null,
  ...over,
});

describe("copy as curl", () => {
  test("a quote in a value is closed, escaped and opened again", () => {
    expect(quote(`it's $(rm -rf /)`)).toBe(`'it'\\''s $(rm -rf /)'`);
  });

  test("the method, the address, the headers and a text body are each one argument", () => {
    const made = curl(
      { method: "POST", path: "/orders" },
      input({
        query: "a=1",
        headers: [
          ["Host", "localhost:8080"],
          ["Content-Length", "9"],
          ["Authorization", "[redacted]"],
          ["X-Note", "it's"],
        ],
        body: body({ text: `{"id":7}` }),
      }),
    );
    expect(made.command).toBe(
      `curl -X 'POST' 'http://localhost:8080/orders?a=1' -H 'Authorization: [redacted]' -H 'X-Note: it'\\''s' --data-raw '{"id":7}'`,
    );
    expect(made.bodyLeftOut).toBe(false);
  });

  test("a binary, cut or withheld body is left out and says so", () => {
    const row = { method: "PUT", path: "/files" };
    expect(curl(row, input({ body: body({ text: null, base64: "AAE=" }) }))).toEqual({ command: `curl -X 'PUT' 'http://localhost/files'`, bodyLeftOut: true });
    expect(curl(row, input({ body: body({ text: "abc", cut: true }) })).bodyLeftOut).toBe(true);
    expect(curl(row, input({ body: body({ text: null, withheld: true }) })).bodyLeftOut).toBe(true);
    expect(curl({ method: "GET", path: "/" }, input()).bodyLeftOut).toBe(false);
  });
});
