// A language client the size of the protocol: framing, request ids, and nothing else.
//
// The protocol suite drives the real `nvs lsp` binary over stdio from plain Node, which is the one
// place the two halves of `rule:ide/one-server-two-thin-clients` are checked against each other
// without an editor in the room — the extension-host tier needs a display and is CI's
// (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`).
//
// `vscode-jsonrpc` would do the framing, and is deliberately not used: it arrives here only as a
// transitive dependency of the client, and a suite that proves the wire is right should not read
// the wire through the same library the extension does. The forty lines below are the whole of
// LSP's base protocol.

import { ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";

const ROOT = resolve(__dirname, "..", "..", "..", "..", "..");

// How long any one answer may take. Generous because the binary under test is usually the debug
// build the loop driver leaves behind, and a first analysis on a cold process is not fast.
const DEADLINE = 30_000;

export interface Position {
  line: number;
  character: number;
}

export interface Range {
  start: Position;
  end: Position;
}

export interface Diagnostic {
  range: Range;
  severity?: number;
  code?: string;
  source?: string;
  message: string;
}

export interface ServerInfo {
  name: string;
  version?: string;
}

export interface Legend {
  tokenTypes: string[];
  tokenModifiers: string[];
}

export interface InitializeResult {
  capabilities: {
    positionEncoding?: string;
    textDocumentSync?: unknown;
    semanticTokensProvider?: { legend: Legend };
    [capability: string]: unknown;
  };
  serverInfo?: ServerInfo;
}

interface Message {
  id?: number;
  method?: string;
  params?: unknown;
  result?: unknown;
  error?: { code: number; message: string };
}

/**
 * The `nvs` binary this suite drives.
 *
 * `NVS_BIN` wins, so CI can point the suite at whatever it just built. Otherwise the repository's
 * own `target/` is preferred over `PATH`: a developer with an older `nvs` installed should still be
 * testing the tree they are editing, which is the same mistake
 * `rule:ide/the-extension-refuses-a-binary-it-does-not-understand` exists to catch in an editor.
 */
export function binary(): string {
  const named = process.env.NVS_BIN;
  if (named !== undefined && named.length > 0) {
    return named;
  }
  const executable = process.platform === "win32" ? "nvs.exe" : "nvs";
  for (const profile of ["debug", "release"]) {
    const built = join(ROOT, "target", profile, executable);
    if (existsSync(built)) {
      return built;
    }
  }
  return "nvs";
}

/** One `nvs lsp` process, from `initialize` to `exit`. */
export class Session {
  private readonly child: ChildProcessWithoutNullStreams;
  private readonly waiting = new Map<number, (message: Message) => void>();
  private readonly notifications: Message[] = [];
  private readonly watchers: ((message: Message) => boolean)[] = [];
  private buffer = Buffer.alloc(0);
  private stderr = "";
  private next = 0;

  private constructor(child: ChildProcessWithoutNullStreams) {
    this.child = child;
    this.child.stdout.on("data", (chunk: Buffer) => this.receive(chunk));
    this.child.stderr.on("data", (chunk: Buffer) => {
      this.stderr += chunk.toString("utf8");
    });
  }

  /**
   * Spawn the server and complete the handshake, returning what `initialize` declared.
   *
   * `args` is what a client appends after `lsp`, and defaults to what the extension sends, which
   * is nothing. It is a parameter because the argv is the half of the launch no other tier sees: a
   * client library may add a flag of its own from a field that reads like transport configuration,
   * and a suite that always spawned the ideal command line would stay green while the pair that
   * ships could not start.
   */
  static async start(args: string[] = []): Promise<{ session: Session; declared: InitializeResult }> {
    const child = spawn(binary(), ["lsp", ...args], { stdio: ["pipe", "pipe", "pipe"] });
    const session = new Session(child);
    const declared = await session.request<InitializeResult>("initialize", {
      processId: process.pid,
      rootUri: null,
      capabilities: {},
    });
    session.notify("initialized", {});
    return { session, declared };
  }

  /** Send a request and resolve with its result, or reject with the error the server answered. */
  request<T>(method: string, params: unknown): Promise<T> {
    const id = ++this.next;
    return this.deadline(
      new Promise<T>((resolve_, reject) => {
        this.waiting.set(id, (message) => {
          if (message.error !== undefined) {
            reject(new Error(`${method}: ${message.error.message}`));
          } else {
            resolve_(message.result as T);
          }
        });
        this.send({ jsonrpc: "2.0", id, method, params });
      }),
      method,
    );
  }

  notify(method: string, params: unknown): void {
    this.send({ jsonrpc: "2.0", method, params });
  }

  open(uri: string, text: string): void {
    this.notify("textDocument/didOpen", {
      textDocument: { uri, languageId: "nvs", version: 1, text },
    });
  }

  /**
   * The next diagnostics published for `uri`, including one that arrived before this was called.
   *
   * A publication is a notification, so it is not ordered against any request: a suite that read
   * the wire only while waiting for an answer would race the analysis and pass or fail by timing.
   */
  diagnostics(uri: string): Promise<Diagnostic[]> {
    return this.deadline(
      new Promise<Diagnostic[]>((resolve_) => {
        const matches = (message: Message): boolean => {
          const params = message.params as { uri: string; diagnostics: Diagnostic[] } | undefined;
          if (message.method !== "textDocument/publishDiagnostics" || params?.uri !== uri) {
            return false;
          }
          resolve_(params.diagnostics);
          return true;
        };
        const already = this.notifications.findIndex(matches);
        if (already >= 0) {
          this.notifications.splice(already, 1);
        } else {
          this.watchers.push(matches);
        }
      }),
      `diagnostics for ${uri}`,
    );
  }

  /**
   * Shut the server down the way an editor does, and answer the exit code it left.
   *
   * Closing stdin is part of the sequence and not a courtesy: the server's reader thread ends at
   * the `exit` notification, and the process is only gone once the descriptors are.
   */
  async close(): Promise<number | null> {
    await this.request("shutdown", null);
    this.notify("exit", null);
    this.child.stdin.end();
    return this.deadline(
      new Promise<number | null>((resolve_) => {
        if (this.child.exitCode !== null) {
          resolve_(this.child.exitCode);
        } else {
          this.child.on("exit", (code) => resolve_(code));
        }
      }),
      "exit",
    );
  }

  /** Kill the process whatever state it is in, so a failed assertion leaves nothing behind. */
  kill(): void {
    this.child.kill();
  }

  /** Whatever the server wrote to stderr, which is where a panic would be. */
  errors(): string {
    return this.stderr;
  }

  private deadline<T>(promise: Promise<T>, what: string): Promise<T> {
    let timer: NodeJS.Timeout;
    const expiry = new Promise<never>((_, reject) => {
      timer = setTimeout(() => {
        reject(new Error(`${what}: no answer in ${DEADLINE}ms. stderr: ${this.stderr || "(empty)"}`));
      }, DEADLINE);
    });
    return Promise.race([promise, expiry]).finally(() => clearTimeout(timer));
  }

  private send(message: unknown): void {
    const body = Buffer.from(JSON.stringify(message), "utf8");
    this.child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
    this.child.stdin.write(body);
  }

  private receive(chunk: Buffer): void {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    for (;;) {
      const head = this.buffer.indexOf("\r\n\r\n");
      if (head < 0) {
        return;
      }
      const header = this.buffer.subarray(0, head).toString("ascii");
      const length = /content-length: *(\d+)/i.exec(header)?.[1];
      if (length === undefined) {
        throw new Error(`a frame with no Content-Length: ${header}`);
      }
      const start = head + 4;
      const end = start + Number(length);
      if (this.buffer.length < end) {
        return;
      }
      const message = JSON.parse(this.buffer.subarray(start, end).toString("utf8")) as Message;
      this.buffer = this.buffer.subarray(end);
      this.dispatch(message);
    }
  }

  private dispatch(message: Message): void {
    const answer = message.id === undefined ? undefined : this.waiting.get(message.id);
    if (answer !== undefined) {
      this.waiting.delete(message.id as number);
      answer(message);
      return;
    }
    const watcher = this.watchers.findIndex((wants) => wants(message));
    if (watcher >= 0) {
      this.watchers.splice(watcher, 1);
    } else {
      this.notifications.push(message);
    }
  }
}
