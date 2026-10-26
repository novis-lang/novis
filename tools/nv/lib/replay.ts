// A `.replay`'s session with `nvs lsp`, and the `.replay.out` it records. `tools/nv/cmd/site.ts`
// § *A snippet* is the home of both formats; this module reads the first, sends the server the session
// its steps describe, and writes what the server returned in the second.
//
// The session is one stream written to the server's stdin, as the `.tainted` one is, so everything in it
// is decided before the server answers. Four things in it depend on an answer: which characters are
// trigger characters, from the `initialize` result; the text an `accept` writes, from the list it
// accepts from; the edits a `rename` or an `action` makes; and the method a `calls` step asks about,
// from the `prepareCallHierarchy` answer. `recordReplay` guesses each, plays the session, and plays it
// again with what the server returned until the guesses are right. That takes two sessions for most
// replays, and one more for each step after an edit the guess did not know.

import { join } from "node:path";
import { lspFrame, lspMessages } from "./lsp.ts";
import { ROOT } from "./paths.ts";

/** One step of a `.replay`. */
export type Step =
  | { kind: "type"; text: string }
  | { kind: "accept"; label: string }
  | { kind: "rename"; name: string }
  | { kind: "action"; title: string }
  | { kind: "hover" | "card" | "lens" | "calls" };

/** A parsed `.replay`: the text the session opens on, the cursor's offset in it, and the steps. */
export interface Replay {
  start: string;
  cursor: number;
  steps: Step[];
}

/** An edit as the page reads one: `[line, col, endLine, endCol, text]`, positions from 0. */
export type Edit = [number, number, number, number, string];

/** The `.replay` parser and the edits the page uses too: `website/config/replay-frames.mjs`, imported by
 * path so `tsc` reads no `.mjs` of the site's. `parseReplay` returns a `Replay`, or why the text is not
 * one. */
const frames = (await import(join(ROOT, "website/config/replay-frames.mjs"))) as {
  CURSOR: string;
  parseReplay(text: string): Replay | string;
  applyEdits(text: string, cursor: number, edits: Edit[]): { text: string; cursor: number };
};
export const CURSOR = frames.CURSOR;
export const parseReplay = frames.parseReplay;

/** What `replaySession` assumes about the server's answers before it has them. Each record is by step
 * index from 0, and has the steps it is known for. */
export interface Guess {
  triggers: string[];
  /** The text each `accept` step writes. */
  inserts: Record<number, string>;
  /** The edits each `rename` and `action` step makes in the open file. */
  edits: Record<number, Edit[]>;
  /** The call hierarchy item each `calls` step asks about. */
  items: Record<number, object>;
}

/** A request in the session whose answer goes into the recording. A `calls` ask has the ids of its two
 * call requests once the guess knows the item they name. */
export type Ask =
  | { kind: "completion" | "diagnostic" | "card" | "rename" | "lens"; step: number; id: number; line: number; character: number }
  | { kind: "action"; step: number; id: number; line: number; character: number; title: string }
  | { kind: "calls"; step: number; id: number; line: number; character: number; calls: { incoming: number; outgoing: number } | null };

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
  const send = (method: string, params: object) => {
    messages.push({ id, method, params: { textDocument: { uri }, ...params } });
    return id++;
  };
  // A request at the cursor, and the ask that records its answer.
  const request = (kind: "completion" | "diagnostic" | "card" | "rename" | "lens", step: number, method: string, params: object = {}) => {
    const at = position(text, cursor);
    asks.push({ kind, step, id: send(method, { position: at, ...params }), ...at });
  };
  const edit = (step: number) => {
    const edits = guess.edits[step];
    if (edits === undefined) return;
    ({ text, cursor } = frames.applyEdits(text, cursor, edits));
    change();
  };
  for (const [step, s] of replay.steps.entries()) {
    if (s.kind === "type") {
      for (const c of s.text) {
        text = text.slice(0, cursor) + c + text.slice(cursor);
        cursor += c.length;
        change();
        if (guess.triggers.includes(c)) {
          list = { ask: asks.length, from: cursor };
          request("completion", step, "textDocument/completion", { context: { triggerKind: 2, triggerCharacter: c } });
        } else if (list !== null && nameChar(c)) continue;
        else if (/[A-Za-z_]/.test(c) && !nameChar(text[cursor - 2])) {
          list = { ask: asks.length, from: cursor - 1 };
          request("completion", step, "textDocument/completion", { context: { triggerKind: 1 } });
        } else list = null;
      }
      continue;
    }
    if (s.kind === "accept") {
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
      continue;
    }
    if (s.kind === "hover") {
      request("diagnostic", step, "textDocument/hover");
      continue;
    }
    list = null;
    const at = position(text, cursor);
    if (s.kind === "card") request("card", step, "textDocument/hover");
    else if (s.kind === "lens") asks.push({ kind: "lens", step, id: send("textDocument/codeLens", {}), ...at });
    else if (s.kind === "rename") {
      request("rename", step, "textDocument/rename", { newName: s.name });
      edit(step);
    } else if (s.kind === "action") {
      const range = { start: at, end: at };
      asks.push({ kind: "action", step, id: send("textDocument/codeAction", { range, context: { diagnostics: [] } }), ...at, title: s.title });
      edit(step);
    } else {
      const ask = send("textDocument/prepareCallHierarchy", { position: at });
      const item = guess.items[step];
      const calls = item === undefined ? null : { incoming: send("callHierarchy/incomingCalls", { item }), outgoing: send("callHierarchy/outgoingCalls", { item }) };
      asks.push({ kind: "calls", step, id: ask, ...at, calls });
    }
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

interface Pos {
  line: number;
  character: number;
}

interface Diagnostic {
  range: { start: Pos; end: Pos };
  code?: string | number;
  message: string;
}

interface TextEdit {
  range: { start: Pos; end: Pos };
  newText: string;
}

/** A workspace edit, in either of its two forms. A file operation has no `edits`. */
interface WorkspaceEdit {
  changes?: Record<string, TextEdit[]>;
  documentChanges?: { textDocument?: { uri: string }; edits?: TextEdit[]; kind?: string; oldUri?: string; uri?: string }[];
}

/** The text edits of `edit` by the URI of each file it changes. */
function editsByFile(edit: WorkspaceEdit | null | undefined): Map<string, TextEdit[]> {
  const files = new Map<string, TextEdit[]>();
  for (const [file, edits] of Object.entries(edit?.changes ?? {})) files.set(file, edits);
  for (const op of edit?.documentChanges ?? []) {
    const file = op.textDocument?.uri ?? op.oldUri ?? op.uri;
    if (file !== undefined) files.set(file, [...(files.get(file) ?? []), ...(op.edits ?? [])]);
  }
  return files;
}

const editOf = (e: TextEdit): Edit => [e.range.start.line, e.range.start.character, e.range.end.line, e.range.end.character, e.newText];

/** An edit's line in a `.replay.out`: its range from 1 and its new text as a JSON string. */
const editLine = ([l, c, el, ec, t]: Edit) => `edit ${l + 1}:${c + 1}-${el + 1}:${ec + 1} ${JSON.stringify(t)}`;

/** Text lines indented two spaces, a blank line left empty so trailing white space is never written. */
const indented = (text: string) => text.split("\n").map((l) => (l.trim() === "" ? "" : `  ${l}`));

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
  const was = session.guess;
  const next: Guess = { triggers: init.capabilities?.completionProvider?.triggerCharacters ?? [], inserts: { ...was.inserts }, edits: { ...was.edits }, items: { ...was.items } };
  const lists: Row[][] = [];
  // What each accept writes, from the lists answered so far. `strict` fails on one that selects another row.
  const learnAccepts = (strict: boolean): string | null => {
    for (const a of session.accepts) {
      if (a.ask < 0) {
        if (strict) return `step ${a.step + 1}: \`accept ${a.label}\` with no list open`;
        continue;
      }
      const rows = lists[a.ask];
      if (rows === undefined) continue;
      const selected = rows[narrow(rows.map((r) => r.label), a.typed)[0]?.index ?? -1];
      if (selected?.label === a.label) next.inserts[a.step] = selected.insert;
      else if (strict) return `step ${a.step + 1}: after \`${a.typed}\` the list selects ${selected ? `\`${selected.label}\`` : "nothing"}, not \`${a.label}\``;
    }
    return null;
  };
  // A failure keeps what this session learned, so a step that failed on a wrong guess is played again.
  const fail = (failure: string) => (learnAccepts(false), { failure, next });
  const answer = (step: number, id: number): { result: unknown } | string => {
    const got = messages[answerAt(id)];
    if (got === undefined) return `step ${step + 1}: \`nvs lsp\` sent no answer to request ${id}`;
    if (got.error !== undefined) return `step ${step + 1}: \`nvs lsp\` answered request ${id} with an error: ${(got.error as { message?: string }).message}`;
    return { result: got.result };
  };
  const blocks: string[] = [];
  for (const ask of session.asks) {
    const got = answer(ask.step, ask.id);
    if (typeof got === "string") return fail(got);
    const where = `${ask.line + 1}:${ask.character + 1}`;
    const head = `${ask.kind} ${ask.step + 1} ${where}`;
    if (ask.kind === "completion") {
      const result = got.result as Item[] | { items: Item[] } | null;
      const rows = (Array.isArray(result) ? result : (result?.items ?? [])).map(row);
      lists.push(rows);
      blocks.push([head, ...rows.map(rowLine)].join("\n"));
      continue;
    }
    lists.push([]);
    if (ask.kind === "diagnostic") {
      const at = answerAt(ask.id);
      const published = messages
        .slice(0, at)
        .filter((m) => m.method === "textDocument/publishDiagnostics" && (m.params as { uri: string }).uri === uri)
        .at(-1);
      const under = ((published?.params as { diagnostics: Diagnostic[] } | undefined)?.diagnostics ?? []).find((d) => before(d.range.start, ask) && before(ask, d.range.end));
      if (under === undefined) return fail(`step ${ask.step + 1}: no diagnostic under the cursor at ${where}`);
      const { start, end } = under.range;
      if (start.line !== end.line) return fail(`step ${ask.step + 1}: the diagnostic under the cursor spans more than one line`);
      blocks.push([`diagnostic ${ask.step + 1} ${start.line + 1}:${start.character + 1}+${end.character - start.character} ${under.code ?? "-"}`, ...under.message.split("\n").map((l) => `  ${l}`)].join("\n"));
    } else if (ask.kind === "card") {
      const contents = (got.result as { contents?: unknown } | null)?.contents;
      const parts = (Array.isArray(contents) ? contents : contents === undefined ? [] : [contents]).map((c) => (typeof c === "string" ? c : ((c as { value?: string }).value ?? "")));
      const markdown = parts.join("\n\n").trim();
      if (markdown === "") return fail(`step ${ask.step + 1}: no hover card at ${where}`);
      blocks.push([head, ...indented(markdown)].join("\n"));
    } else if (ask.kind === "rename") {
      const files = editsByFile(got.result as WorkspaceEdit | null);
      const own = (files.get(uri) ?? []).map(editOf);
      const others = [...files.keys()].filter((f) => f !== uri).length;
      if (own.length === 0 && others === 0) return fail(`step ${ask.step + 1}: nothing to rename at ${where}`);
      next.edits[ask.step] = own;
      blocks.push([`${head} ${others}`, ...own.map(editLine)].join("\n"));
    } else if (ask.kind === "action") {
      const actions = (got.result as { title: string; edit?: WorkspaceEdit }[] | null) ?? [];
      const chosen = actions.find((a) => a.title === ask.title);
      if (chosen === undefined) return fail(`step ${ask.step + 1}: the code actions at ${where} are ${actions.map((a) => `"${a.title}"`).join(", ") || "none"}, not "${ask.title}"`);
      const own = (editsByFile(chosen.edit).get(uri) ?? []).map(editOf);
      next.edits[ask.step] = own;
      blocks.push([head, ...actions.map((a) => `  ${a.title}`), ...own.map(editLine)].join("\n"));
    } else if (ask.kind === "lens") {
      const lenses = (got.result as { range: { start: Pos }; command?: { title: string } }[] | null) ?? [];
      if (lenses.length === 0) return fail(`step ${ask.step + 1}: the file has no code lenses`);
      blocks.push([`lens ${ask.step + 1}`, ...lenses.map((l) => `  ${l.range.start.line + 1}:${l.range.start.character + 1} ${l.command?.title ?? ""}`)].join("\n"));
    } else if (ask.kind === "calls") {
      const items = (got.result as { name: string }[] | null) ?? [];
      if (items[0] === undefined) return fail(`step ${ask.step + 1}: no method under the cursor at ${where}`);
      next.items[ask.step] = items[0];
      if (ask.calls === null) return fail(`step ${ask.step + 1}: the calls are asked in the next session`);
      const incoming = answer(ask.step, ask.calls.incoming);
      const outgoing = answer(ask.step, ask.calls.outgoing);
      if (typeof incoming === "string") return fail(incoming);
      if (typeof outgoing === "string") return fail(outgoing);
      const line = (way: string, item: { name: string }, ranges: unknown[]) => `  ${way} ${item.name} ${ranges.length}`;
      blocks.push(
        [
          `${head} ${items[0].name}`,
          ...((incoming.result as { from: { name: string }; fromRanges: unknown[] }[] | null) ?? []).map((c) => line("in", c.from, c.fromRanges)),
          ...((outgoing.result as { to: { name: string }; fromRanges: unknown[] }[] | null) ?? []).map((c) => line("out", c.to, c.fromRanges)),
        ].join("\n"),
      );
    }
  }
  const wrong = learnAccepts(true);
  if (wrong !== null) return { failure: wrong, next };
  return { recording: blocks.length ? `${blocks.join("\n\n")}\n` : "", failure: null, next };
}

const before = (a: Pos, b: Pos) => a.line < b.line || (a.line === b.line && a.character <= b.character);

const sameGuess = (a: Guess, b: Guess) => JSON.stringify([[...a.triggers].sort(), a.inserts, a.edits, a.items]) === JSON.stringify([[...b.triggers].sort(), b.inserts, b.edits, b.items]);

/** The `.replay.out` text `replay` records, from sessions `play` runs, and the text its last step ends
 * on; or why there is none. `play` runs `nvs lsp` with the stream on stdin. */
export async function recordReplay(uri: string, replay: Replay, play: (stream: string) => Promise<{ code: number; stdout: string; stderr: string }>): Promise<{ recording: string; last: string } | { failure: string }> {
  let guess: Guess = { triggers: [], inserts: {}, edits: {}, items: {} };
  // Each round learns the trigger characters or at least one more answer the steps depend on, so this ends.
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
