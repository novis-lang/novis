// `bun nv lsp-start`: how long the language server takes to start on a tree, what work its index did, and
// how much memory it holds.
//
//     bun nv lsp-start <root>                  start `target/release/nvs lsp` on <root> and report
//     bun nv lsp-start <root> --open <file>    also open <file> and time its first `documentSymbol` answer
//     bun nv lsp-start <root> --cold           delete the index file first, so the index is built whole
//     bun nv lsp-start --copies <n>            write a tree of <n> copies of `apps/registry/src` and measure it
//     bun nv lsp-start --copies <n> --edits <m>  then make <m> edits to one library file and measure again
//
// It speaks the protocol over stdio as an editor does: `initialize` with `window.workDoneProgress`,
// `initialized`, then `didOpen` and `textDocument/documentSymbol` for `--open`. It answers the server's
// `window/workDoneProgress/create`, `client/registerCapability` and `workspace/configuration`. It prints
// three times, each from the start of the process: to the `initialize` answer, to the first
// `documentSymbol` answer, and to the indexing progress's `end`. Then it prints the line the server logs
// at that `end`: the index's `Work` since the start (files parsed and checked, and the five file-system
// counts of `nvs_diagnostics::work`) and the file-system work of the server's whole thread, the tree's
// survey included. The counts are the figures to compare, since they are the same on every machine; the
// times are a note (`rule:testing/bench-counters`).
//
// After the indexing `end` it prints the server's memory now and at its peak: on Windows `Get-Process`'s
// `PrivateMemorySize64` and `PeakPagedMemorySize64`, on Linux `/proc/<pid>/status`'s `VmRSS` and `VmHWM`.
// Another system prints no figure. Beside it, it prints the CPU time the server's threads spent since the
// start: `Get-Process`'s `TotalProcessorTime` on Windows, `/proc/<pid>/stat`'s `utime` and `stime` on Linux.
//
// `--copies <n>` writes `.agent-tmp/lsp-start/tree/`: `src/C<k>/` for k from 1 to n, each a copy of
// `apps/registry/src` in the namespace `App\C<k>`, one entry `public/c<k>.nvs` per copy, which is the
// registry's `public/index.nvs` autoloading `App` from `../src`, and the registry's `nvs.toml`. Without a
// root, it measures that tree. `--edits <m>` needs that tree. It opens `src/C1/Publish.nvs`, and for each
// edit adds one method to its class, waits for the `documentSymbol` answer and for the next `index work:`
// line, which the run an edit's queued entries begin ends with. Then it prints the work line and the two
// memory figures again.
//
// The server runs with a data folder of its own, `.agent-tmp/lsp-start/data/`, so a measurement never
// reads or writes the data folder an editor uses, and its index file lasts from one run to the next:
// a run without `--cold` is a warm start. `--cold` deletes that folder's `cache/lsp-index/`. The
// binary is the release build an editor runs, so `bun nv relink --build` first when the tree changed.

import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { ROOT, rel } from "../lib/paths.ts";
import { releaseCli } from "../lib/relink.ts";
import { run as runProc } from "../lib/proc.ts";

export const summary =
  "time the language server's start on a tree and print its index's work counts and memory: nv lsp-start [<root>] [--open <file>] [--cold] [--copies <n>] [--edits <m>]";

const DATA = join(ROOT, ".agent-tmp", "lsp-start", "data");
const TREE = join(ROOT, ".agent-tmp", "lsp-start", "tree");
const REGISTRY = join(ROOT, "apps", "registry");
/** Longer than a cold index of this repository takes on a slow runner. */
const TIMEOUT_MS = 20 * 60 * 1000;

interface Args {
  root: string;
  open: string | null;
  cold: boolean;
  copies: number | null;
  edits: number;
}

function count(flag: string, value: string | undefined): number | string {
  const n = Number(value);
  if (value === undefined || !Number.isInteger(n) || n < 1) return `${flag} needs a whole number above zero`;
  return n;
}

function parse(args: string[]): Args | string {
  let root: string | null = null;
  let open: string | null = null;
  let cold = false;
  let copies: number | null = null;
  let edits = 0;
  for (let i = 0; i < args.length; i++) {
    const arg = args[i]!;
    if (arg === "--cold") cold = true;
    else if (arg === "--open") {
      const next = args[++i];
      if (next === undefined) return "--open needs a file";
      open = resolve(next);
    } else if (arg === "--copies" || arg === "--edits") {
      const n = count(arg, args[++i]);
      if (typeof n === "string") return n;
      if (arg === "--copies") copies = n;
      else edits = n;
    } else if (arg.startsWith("-")) return `unknown argument ${arg}`;
    else if (root === null) root = resolve(arg);
    else return `a second root ${arg}`;
  }
  if (root === null && copies === null) return "a root directory or --copies is needed";
  if (edits > 0 && (copies === null || root !== null)) return "--edits needs the tree --copies writes, and no root";
  return { root: root ?? TREE, open, cold, copies, edits };
}

/** Writes the tree of `copies` copies of the registry's library under `TREE`, replacing any earlier one. */
function writeTree(copies: number): void {
  rmSync(TREE, { recursive: true, force: true });
  const library = join(REGISTRY, "src");
  const files = readdirSync(library).filter((f) => f.endsWith(".nvs"));
  const entry = readFileSync(join(REGISTRY, "public", "index.nvs"), "utf8").replace(
    "autoload 'Registry' from '../src';",
    "autoload 'App' from '../src';",
  );
  mkdirSync(join(TREE, "public"), { recursive: true });
  copyFileSync(join(REGISTRY, "nvs.toml"), join(TREE, "nvs.toml"));
  for (let k = 1; k <= copies; k++) {
    const space = `App\\C${k}`;
    const into = (text: string) =>
      text.replaceAll("namespace Registry;", `namespace ${space};`).replaceAll("Registry\\", `${space}\\`);
    const dir = join(TREE, "src", `C${k}`);
    mkdirSync(dir, { recursive: true });
    for (const file of files) writeFileSync(join(dir, file), into(readFileSync(join(library, file), "utf8")));
    writeFileSync(join(TREE, "public", `c${k}.nvs`), into(entry));
  }
}

const mb = (bytes: number) => `${(bytes / 1024 / 1024).toFixed(1)} MB`;

/** The process's memory now and at its peak, as one line, or why there is no figure. */
function memory(pid: number): string {
  if (process.platform === "win32") {
    const script = `$p = Get-Process -Id ${pid}; "$($p.PrivateMemorySize64) $($p.PeakPagedMemorySize64)"`;
    const out = Bun.spawnSync(["powershell", "-NoProfile", "-Command", script]).stdout.toString().trim();
    const [now, peak] = out.split(/\s+/).map(Number);
    if (!Number.isFinite(now) || !Number.isFinite(peak)) return `no figure: Get-Process printed ${JSON.stringify(out)}`;
    return `private ${mb(now!)}, peak paged ${mb(peak!)}`;
  }
  if (process.platform === "linux") {
    const status = readFileSync(`/proc/${pid}/status`, "utf8");
    const kb = (key: string) => Number(new RegExp(`^${key}:\\s+(\\d+) kB`, "m").exec(status)?.[1]) * 1024;
    return `resident ${mb(kb("VmRSS"))}, peak resident ${mb(kb("VmHWM"))}`;
  }
  return `no figure on ${process.platform}`;
}

/** The CPU time every thread of the process spent so far, as one line, or why there is no figure. */
function cpu(pid: number): string {
  if (process.platform === "win32") {
    // Whole milliseconds, so the figure has no decimal separator for the system's locale to choose.
    const script = `[int64](Get-Process -Id ${pid}).TotalProcessorTime.TotalMilliseconds`;
    const out = Bun.spawnSync(["powershell", "-NoProfile", "-Command", script]).stdout.toString().trim();
    const ms = Number(out);
    if (!Number.isFinite(ms) || out === "") return `no figure: Get-Process printed ${JSON.stringify(out)}`;
    return `${(ms / 1000).toFixed(2)} s`;
  }
  if (process.platform === "linux") {
    // The fields after the command's closing parenthesis; `utime` and `stime` are the 14th and 15th, in
    // clock ticks of 1/100 s on every Linux this runs on.
    const stat = readFileSync(`/proc/${pid}/stat`, "utf8");
    const fields = stat.slice(stat.lastIndexOf(")") + 2).split(" ");
    return `${((Number(fields[11]) + Number(fields[12])) / 100).toFixed(2)} s`;
  }
  return `no figure on ${process.platform}`;
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
  if (parsed.copies !== null && parsed.root === TREE) writeTree(parsed.copies);
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

/** The id of the `documentSymbol` request after edit `n`. */
const EDIT_ID = 1000;

async function measure(exe: string, args: Args): Promise<number> {
  const started = performance.now();
  const at = () => `${((performance.now() - started) / 1000).toFixed(2)} s`;
  const rootUri = pathToFileURL(args.root).href;
  const openUri = args.open === null ? null : pathToFileURL(args.open).href;
  const editPath = join(args.root, "src", "C1", "Publish.nvs");
  const editUri = pathToFileURL(editPath).href;
  const editText = args.edits > 0 ? readFileSync(editPath, "utf8") : "";
  let pending = Buffer.alloc(0);
  let stdin: { write(text: string): void; end(): void; pid: number } | null = null;
  const report: string[] = [];
  let work: string | null = null;
  let symbolsDone = openUri === null;
  let indexed = false;
  let closing = false;
  // The edit being waited on, from 1, and 0 before the first. Each is done once its answer and its work line
  // have both come.
  let edit = 0;
  let editAnswered = false;
  let editWork: string | null = null;

  const send = (message: Json) => {
    const body = JSON.stringify({ jsonrpc: "2.0", ...message });
    stdin?.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
  };
  const close = () => {
    closing = true;
    send({ id: 99, method: "shutdown" });
  };
  /** The edited file's text after `n` edits: `n` methods before the last `}`, which closes its class. */
  const edited = (n: number) => {
    const end = editText.lastIndexOf("}");
    let methods = "";
    for (let j = 1; j <= n; j++) methods += `    public function lspStartEdit${j}(): int { return ${j}; }\n`;
    return `${editText.slice(0, end)}${methods}${editText.slice(end)}`;
  };
  const nextEdit = () => {
    edit += 1;
    editAnswered = false;
    editWork = null;
    send({
      method: "textDocument/didChange",
      params: { textDocument: { uri: editUri, version: edit + 1 }, contentChanges: [{ text: edited(edit) }] },
    });
    send({ id: EDIT_ID + edit, method: "textDocument/documentSymbol", params: { textDocument: { uri: editUri } } });
  };
  const step = () => {
    if (closing) return;
    if (edit === 0) {
      if (!indexed || !symbolsDone || work === null) return;
      report.push(`memory after indexing: ${memory(stdin!.pid)}`);
      report.push(`cpu time after indexing: ${cpu(stdin!.pid)}`);
      if (args.edits === 0) return close();
      send({
        method: "textDocument/didOpen",
        params: { textDocument: { uri: editUri, languageId: "nvs", version: 1, text: editText } },
      });
      return nextEdit();
    }
    if (!editAnswered || editWork === null) return;
    if (edit < args.edits) return nextEdit();
    report.push(`after ${edit} edits:       ${at()}`);
    report.push(editWork);
    report.push(`memory after ${edit} edits: ${memory(stdin!.pid)}`);
    report.push(`cpu time after ${edit} edits: ${cpu(stdin!.pid)}`);
    close();
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
      if (value.kind === "end" && !indexed) {
        report.push(`indexing ended:        ${at()}`);
        indexed = true;
        step();
      }
      return;
    }
    if (method === "window/logMessage") {
      const text = (message.params as Json).message as string;
      if (!text.startsWith("index work:")) return;
      if (edit === 0) {
        if (work === null) {
          work = text;
          report.push(text);
        }
      } else editWork = text;
      step();
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
      step();
    } else if (message.id === EDIT_ID + edit) {
      editAnswered = true;
      step();
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
  if (!closing) {
    const why = result.hung ? "it hung" : result.timedOut ? "it timed out" : `it exited ${result.code}`;
    const waited = edit === 0 ? "an indexing end and work line" : `the answer and work line of edit ${edit}`;
    console.error(`nv lsp-start: the server sent no ${waited} before ${why}`);
    const tail = result.stderr.trimEnd().split("\n").slice(-10).join("\n");
    if (tail) console.error(tail);
    return 1;
  }
  return 0;
}
