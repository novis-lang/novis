// `bun nv lsp-start`: how long the language server takes to start on a tree, and what work its index did.
//
//     bun nv lsp-start <root>                  start `target/release/nvs lsp` on <root> and report
//     bun nv lsp-start <root> --open <file>    also open <file> and time its first `documentSymbol` answer
//     bun nv lsp-start <root> --cold           delete the index file first, so the index is built whole
//
// It speaks the protocol over stdio as an editor does: `initialize` with `window.workDoneProgress`,
// `initialized`, then `didOpen` and `textDocument/documentSymbol` for `--open`. It answers the server's
// `window/workDoneProgress/create`, `client/registerCapability` and `workspace/configuration`. It prints
// three times, each from the start of the process: to the `initialize` answer, to the first
// `documentSymbol` answer, and to the indexing progress's `end`. Then it prints the line the server logs
// at that `end`: the index's `Work` (files parsed and checked, and the five file-system counts of
// `nvs_diagnostics::work`) and the file-system work of the server's whole thread, the tree's survey
// included. The counts are the figures to compare, since they are the same on every machine; the times
// are a note (`rule:testing/bench-counters`).
//
// The server runs with a data folder of its own, `.agent-tmp/lsp-start/data/`, so a measurement never
// reads or writes the data folder an editor uses, and its index file lasts from one run to the next:
// a run without `--cold` is a warm start. `--cold` deletes that folder's `cache/lsp-index/`. The
// binary is the release build an editor runs, so `bun nv relink --build` first when the tree changed.

import { existsSync, readFileSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { ROOT, rel } from "../lib/paths.ts";
import { releaseCli } from "../lib/relink.ts";
import { run as runProc } from "../lib/proc.ts";

export const summary = "time the language server's start on a tree and print its index's work counts: nv lsp-start <root> [--open <file>] [--cold]";

const DATA = join(ROOT, ".agent-tmp", "lsp-start", "data");
/** Longer than a cold index of this repository takes on a slow runner. */
const TIMEOUT_MS = 20 * 60 * 1000;

interface Args {
  root: string;
  open: string | null;
  cold: boolean;
}

function parse(args: string[]): Args | string {
  let root: string | null = null;
  let open: string | null = null;
  let cold = false;
  for (let i = 0; i < args.length; i++) {
    const arg = args[i]!;
    if (arg === "--cold") cold = true;
    else if (arg === "--open") {
      const next = args[++i];
      if (next === undefined) return "--open needs a file";
      open = resolve(next);
    } else if (arg.startsWith("-")) return `unknown argument ${arg}`;
    else if (root === null) root = resolve(arg);
    else return `a second root ${arg}`;
  }
  if (root === null) return "a root directory is needed";
  return { root, open, cold };
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const parsed = parse(args);
  if (typeof parsed === "string") {
    console.error(`nv lsp-start: ${parsed}`);
    return 2;
  }
  const exe = releaseCli();
  if (!existsSync(exe)) {
    console.error(`nv lsp-start: ${rel(exe)} does not exist; run \`bun nv relink --build\` first`);
    return 1;
  }
  if (!existsSync(parsed.root)) {
    console.error(`nv lsp-start: ${parsed.root} does not exist`);
    return 2;
  }
  if (parsed.open !== null && !existsSync(parsed.open)) {
    console.error(`nv lsp-start: ${parsed.open} does not exist`);
    return 2;
  }
  if (parsed.cold) rmSync(join(DATA, "cache", "lsp-index"), { recursive: true, force: true });
  return await measure(exe, parsed);
}

type Json = Record<string, unknown>;

async function measure(exe: string, args: Args): Promise<number> {
  const started = performance.now();
  const at = () => `${((performance.now() - started) / 1000).toFixed(2)} s`;
  const rootUri = pathToFileURL(args.root).href;
  const openUri = args.open === null ? null : pathToFileURL(args.open).href;
  let pending = Buffer.alloc(0);
  let stdin: { write(text: string): void; end(): void } | null = null;
  const report: string[] = [];
  let work: string | null = null;
  let symbolsDone = openUri === null;
  let indexed = false;
  let closing = false;

  const send = (message: Json) => {
    const body = JSON.stringify({ jsonrpc: "2.0", ...message });
    stdin?.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
  };
  const finishIfDone = () => {
    if (closing || !indexed || !symbolsDone || work === null) return;
    closing = true;
    send({ id: 99, method: "shutdown" });
  };

  const handle = (message: Json) => {
    const method = message.method as string | undefined;
    if (method !== undefined && message.id !== undefined) {
      // A request from the server: `workspace/configuration` takes one answer per item, and the rest
      // take `null`.
      const items = (message.params as Json | undefined)?.items as unknown[] | undefined;
      send({ id: message.id, result: method === "workspace/configuration" ? (items ?? []).map(() => null) : null });
      return;
    }
    if (method === "$/progress") {
      const value = (message.params as Json).value as Json;
      if (value.kind === "end") {
        report.push(`indexing ended:        ${at()}`);
        indexed = true;
        finishIfDone();
      }
      return;
    }
    if (method === "window/logMessage") {
      const text = (message.params as Json).message as string;
      if (text.startsWith("index work:")) {
        work = text;
        finishIfDone();
      }
      return;
    }
    if (method !== undefined) return;
    if (message.id === 1) {
      report.push(`initialize answered:   ${at()}`);
      send({ method: "initialized", params: {} });
      if (openUri !== null) {
        send({
          method: "textDocument/didOpen",
          params: { textDocument: { uri: openUri, languageId: "nvs", version: 1, text: readFileSync(args.open!, "utf8") } },
        });
        send({ id: 2, method: "textDocument/documentSymbol", params: { textDocument: { uri: openUri } } });
      }
    } else if (message.id === 2) {
      report.push(`documentSymbol answer: ${at()}`);
      symbolsDone = true;
      finishIfDone();
    } else if (message.id === 99) {
      send({ method: "exit" });
      stdin?.end();
    }
  };

  const read = (text: string) => {
    pending = Buffer.concat([pending, Buffer.from(text, "utf8")]);
    for (;;) {
      const head = pending.indexOf("\r\n\r\n");
      if (head < 0) return;
      const length = Number(/Content-Length: (\d+)/i.exec(pending.subarray(0, head).toString("latin1"))?.[1]);
      if (pending.length < head + 4 + length) return;
      const body = pending.subarray(head + 4, head + 4 + length).toString("utf8");
      pending = pending.subarray(head + 4 + length);
      handle(JSON.parse(body) as Json);
    }
  };

  const result = await runProc([exe, "--data", DATA, "lsp"], {
    cwd: args.root,
    timeoutMs: TIMEOUT_MS,
    onText: (text, stream) => {
      if (stream === "stdout") read(text);
    },
    talk: (writer) => {
      stdin = writer;
      send({
        id: 1,
        method: "initialize",
        params: {
          processId: process.pid,
          rootUri,
          workspaceFolders: [{ uri: rootUri, name: "root" }],
          capabilities: { window: { workDoneProgress: true } },
        },
      });
    },
  });
  for (const line of report) console.log(line);
  if (work !== null) console.log(work);
  if (!indexed || work === null) {
    const why = result.hung ? "it hung" : result.timedOut ? "it timed out" : `it exited ${result.code}`;
    console.error(`nv lsp-start: the server sent no indexing end and work line before ${why}`);
    const tail = result.stderr.trimEnd().split("\n").slice(-10).join("\n");
    if (tail) console.error(tail);
    return 1;
  }
  return 0;
}
