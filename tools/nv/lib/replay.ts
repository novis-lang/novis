// A `.replay`'s session with `nvs lsp`, and the `.replay.out` it records. `tools/nv/cmd/site.ts`
// § *A snippet* is the home of both formats; this module reads the first, sends the server the session
// its steps describe, and writes what the server returned in the second.
//
// The session is one stream written to the server's stdin, as the `.tainted` one is, so everything in it
// is decided before the server answers. Two things in it depend on an answer: which characters are
// trigger characters, from the `initialize` result, and the text an `accept` writes, from the list it
// accepts from. `recordReplay` guesses both, plays the session, and plays it again with what the
// server returned until the guesses are right. That takes two sessions for most replays.

import { join } from "node:path";
import { lspFrame, lspMessages } from "./lsp.ts";
import { ROOT } from "./paths.ts";

/** One step of a `.replay`. */
export type Step = { kind: "type"; text: string } | { kind: "accept"; label: string } | { kind: "hover" };

/** A parsed `.replay`: the text the session opens on, the cursor's offset in it, and the steps. */
export interface Replay {
  start: string;
  cursor: number;
  steps: Step[];
}

/** The `.replay` parser the page uses too: `website/config/replay-frames.mjs`, imported by path so `tsc`
 * reads no `.mjs` of the site's. `parseReplay` returns a `Replay`, or why the text is not one. */
const frames = (await import(join(ROOT, "website/config/replay-frames.mjs"))) as { CURSOR: string; parseReplay(text: string): Replay | string };
export const CURSOR = frames.CURSOR;
export const parseReplay = frames.parseReplay;

/** What `replaySession` assumes about the server's answers before it has them. */
export interface Guess {
  triggers: string[];
  /** The text each `accept` step writes, by step index from 0, where it is known. */
  inserts: Record<number, string>;
}

/** A request in the session whose answer goes into the recording. */
export type Ask =
  | { kind: "completion"; step: number; id: number; line: number; character: number }
  | { kind: "diagnostic"; step: number; id: number; line: number; character: number };

/** An `accept` in the session: the list it accepts from, as an index into `asks`, and the letters typed
 * since that list opened. */
export interface Accept {
  step: number;
  label: string;
  ask: number;
  typed: string;
}

/** The session a replay's steps describe, under `guess`. An `accept` with no list open has `ask` -1. */
export interface Session {
  stream: string;
  guess: Guess;
  asks: Ask[];
  accepts: Accept[];
  /** The text after the last step. */
  last: string;
}

const nameChar = (c: string | undefined) => c !== undefined && /[A-Za-z0-9_]/.test(c);

/** The line and the UTF-16 column, both from 0, of `offset` in `text`. */
function position(text: string, offset: number): { line: number; character: number } {
  const before = text.slice(0, offset).split("\n");
  return { line: before.length - 1, character: before[before.length - 1]!.length };
}

/** The stream `nvs lsp` is sent for `replay` with the document at `uri`. */
export function replaySession(uri: string, replay: Replay, guess: Guess): Session {
  let text = replay.start;
  let cursor = replay.cursor;
  let version = 1;
  let id = 1;
  const messages: object[] = [
    { id: id++, method: "initialize", params: { processId: null, rootUri: null, capabilities: {} } },
    { method: "initialized", params: {} },
    { method: "textDocument/didOpen", params: { textDocument: { uri, languageId: "nvs", version, text } } },
  ];
  const asks: Ask[] = [];
  const accepts: Accept[] = [];
  // The open list: which ask it came from and where the text it narrows on starts.
  let list: { ask: number; from: number } | null = null;
  const change = () => messages.push({ method: "textDocument/didChange", params: { textDocument: { uri, version: ++version }, contentChanges: [{ text }] } });
  const request = (kind: Ask["kind"], step: number, method: string, context?: object) => {
    const at = position(text, cursor);
    messages.push({ id, method, params: { textDocument: { uri }, position: at, ...(context ? { context } : {}) } });
    asks.push({ kind, step, id: id++, ...at });
  };
  for (const [step, s] of replay.steps.entries()) {
    if (s.kind === "type") {
      for (const c of s.text) {
        text = text.slice(0, cursor) + c + text.slice(cursor);
        cursor += c.length;
        change();
        if (guess.triggers.includes(c)) {
          list = { ask: asks.length, from: cursor };
          request("completion", step, "textDocument/completion", { triggerKind: 2, triggerCharacter: c });
        } else if (list !== null && nameChar(c)) continue;
        else if (/[A-Za-z_]/.test(c) && !nameChar(text[cursor - 2])) {
          list = { ask: asks.length, from: cursor - 1 };
          request("completion", step, "textDocument/completion", { triggerKind: 1 });
        } else list = null;
      }
    } else if (s.kind === "accept") {
      // A guess of the trigger characters can be wrong, so this is named once the server has answered.
      if (list === null) {
        accepts.push({ step, label: s.label, ask: -1, typed: "" });
        continue;
      }
      const insert = guess.inserts[step] ?? s.label;
      accepts.push({ step, label: s.label, ask: list.ask, typed: text.slice(list.from, cursor) });
      text = text.slice(0, list.from) + insert + text.slice(cursor);
      cursor = list.from + insert.length;
      list = null;
      change();
    } else request("diagnostic", step, "textDocument/hover");
  }
  messages.push({ id, method: "shutdown", params: null }, { method: "exit", params: null });
  return { stream: messages.map(lspFrame).join(""), guess, asks, accepts, last: text };
}

/** LSP's completion item kinds, by number, spelled as `tests/lsp/` cases spell them. */
const KINDS = ["text", "method", "function", "constructor", "field", "variable", "class", "interface", "module", "property", "unit", "value", "enum", "keyword", "snippet", "color", "file", "reference", "folder", "enumMember", "constant", "struct", "event", "operator", "typeParameter"];

/** A list row as an editor shows it, read the way `tests/lsp/` cases read one: `right` is the text at
 * the row's right, the item's description where it has label details and its detail otherwise, and
 * `after` is the text written straight after the label. */
interface Row {
  label: string;
  kind: string;
  right: string;
  after: string;
  insert: string;
}

interface Item {
  label: string;
  labelDetails?: { detail?: string; description?: string };
  kind?: number;
  detail?: string;
  insertText?: string;
  textEdit?: { newText: string };
}

const row = (i: Item): Row => ({
  label: i.label,
  kind: i.kind === undefined ? "-" : (KINDS[i.kind - 1] ?? String(i.kind)),
  right: (i.labelDetails ? i.labelDetails.description : i.detail) ?? "",
  after: i.labelDetails?.detail ?? "",
  insert: i.textEdit?.newText ?? i.insertText ?? i.label,
});

/** A row's line in a `.replay.out`, its empty columns at the end left off. */
function rowLine(r: Row): string {
  const columns = [r.label, r.kind, r.right, r.after, r.insert === r.label ? "" : r.insert];
  while (columns.at(-1) === "") columns.pop();
  return columns.join("\t");
}

interface Diagnostic {
  range: { start: { line: number; character: number }; end: { line: number; character: number } };
  code?: string | number;
  message: string;
}

/** The narrowing the page imitates: `website/config/replay-narrow.mjs`, imported by path so `tsc` reads
 * no `.mjs` of the site's. */
const { narrow } = (await import(join(ROOT, "website/config/replay-narrow.mjs"))) as { narrow(labels: string[], typed: string): { index: number; matched: number[] }[] };

/** What one session's standard output, `stdout`, records: the `.replay.out` text, or why the session
 * does not fit the replay. `next` is the guess the answers make. When it differs from the session's
 * guess, the session has to be played again, and neither the recording nor the failure counts. */
export function replayRecording(uri: string, session: Session, stdout: string): { recording: string; failure: null; next: Guess } | { failure: string; next: Guess } {
  const messages = lspMessages(stdout);
  const answerAt = (id: number) => messages.findIndex((m) => m.id === id && !("method" in m));
  const init = messages[answerAt(1)]?.result as { capabilities?: { completionProvider?: { triggerCharacters?: string[] } } } | undefined;
  if (init === undefined) return { failure: "`nvs lsp` sent no `initialize` result", next: session.guess };
  const next: Guess = { triggers: init.capabilities?.completionProvider?.triggerCharacters ?? [], inserts: {} };
  const fail = (failure: string) => ({ failure, next: { ...next, inserts: session.guess.inserts } });
  const blocks: string[] = [];
  const lists: Row[][] = [];
  for (const ask of session.asks) {
    const at = answerAt(ask.id);
    const answer = messages[at];
    if (answer === undefined) return fail(`step ${ask.step + 1}: \`nvs lsp\` sent no answer to request ${ask.id}`);
    if (answer.error !== undefined) return fail(`step ${ask.step + 1}: \`nvs lsp\` answered request ${ask.id} with an error: ${(answer.error as { message?: string }).message}`);
    if (ask.kind === "completion") {
      const result = answer.result as Item[] | { items: Item[] } | null;
      const rows = (Array.isArray(result) ? result : (result?.items ?? [])).map(row);
      lists.push(rows);
      blocks.push([`completion ${ask.step + 1} ${ask.line + 1}:${ask.character + 1}`, ...rows.map(rowLine)].join("\n"));
      continue;
    }
    lists.push([]);
    const published = messages
      .slice(0, at)
      .filter((m) => m.method === "textDocument/publishDiagnostics" && (m.params as { uri: string }).uri === uri)
      .at(-1);
    const under = ((published?.params as { diagnostics: Diagnostic[] } | undefined)?.diagnostics ?? []).find((d) => before(d.range.start, ask) && before(ask, d.range.end));
    if (under === undefined) return fail(`step ${ask.step + 1}: no diagnostic under the cursor at ${ask.line + 1}:${ask.character + 1}`);
    const { start, end } = under.range;
    if (start.line !== end.line) return fail(`step ${ask.step + 1}: the diagnostic under the cursor spans more than one line`);
    blocks.push([`diagnostic ${ask.step + 1} ${start.line + 1}:${start.character + 1}+${end.character - start.character} ${under.code ?? "-"}`, ...under.message.split("\n").map((l) => `  ${l}`)].join("\n"));
  }
  for (const a of session.accepts) {
    if (a.ask < 0) return fail(`step ${a.step + 1}: \`accept ${a.label}\` with no list open`);
    const rows = lists[a.ask]!;
    const selected = rows[narrow(rows.map((r) => r.label), a.typed)[0]?.index ?? -1];
    if (selected?.label !== a.label) return fail(`step ${a.step + 1}: after \`${a.typed}\` the list selects ${selected ? `\`${selected.label}\`` : "nothing"}, not \`${a.label}\``);
    next.inserts[a.step] = selected.insert;
  }
  return { recording: blocks.length ? `${blocks.join("\n\n")}\n` : "", failure: null, next };
}

const before = (a: { line: number; character: number }, b: { line: number; character: number }) => a.line < b.line || (a.line === b.line && a.character <= b.character);

const sameGuess = (a: Guess, b: Guess) => JSON.stringify([[...a.triggers].sort(), a.inserts]) === JSON.stringify([[...b.triggers].sort(), b.inserts]);

/** The `.replay.out` text `replay` records, from sessions `play` runs, and the text its last step ends
 * on; or why there is none. `play` runs `nvs lsp` with the stream on stdin. */
export async function recordReplay(uri: string, replay: Replay, play: (stream: string) => Promise<{ code: number; stdout: string; stderr: string }>): Promise<{ recording: string; last: string } | { failure: string }> {
  let guess: Guess = { triggers: [], inserts: {} };
  // Each round learns the trigger characters or at least one more accepted text, so this ends.
  for (let round = 0; round <= replay.steps.length + 1; round++) {
    const session = replaySession(uri, replay, guess);
    const ran = await play(session.stream);
    const got = replayRecording(uri, session, ran.stdout);
    if (!sameGuess(guess, got.next)) {
      guess = got.next;
      continue;
    }
    if (got.failure === null) return { recording: got.recording, last: session.last };
    return { failure: `${got.failure} (exit ${ran.code}: ${ran.stderr.trim().split("\n")[0] ?? ""})` };
  }
  return { failure: "the session's answers did not settle" };
}
