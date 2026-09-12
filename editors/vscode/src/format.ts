// Formatting a `.nvs` file: `nvs fmt` over the buffer, the editor's own HTML formatter over the
// markup in what it answered, and one edit back.
//
// Neither half is a layout decision of this client's (`rule:ide/one-server-two-thin-clients`).
// `nvs fmt` is the only formatter of Novis there is and its layout is unconfigurable by decision
// (`rule:tooling/fmt-is-one-canonical-style`); the markup bytes are the editor's own formatter's
// (`rule:ide/a-template-region-gets-services-but-no-second-formatter`). What this file decides is
// the order, and the order is ADR 0173 § 1's: `nvs fmt` over the whole buffer, `nvs/regions` over
// what it answered, then one chunk of that markup at a time.
//
// **The buffer goes in over standard input, not the path on disk.** The request is answered for
// the document the user is looking at, which under format-on-save is the text about to be written
// and is otherwise often unsaved; `nvs fmt <path>` would lay out the previous save and hand back a
// rewrite of it. The bytes pass through untouched in both directions, so a buffer becomes exactly
// what `nvs fmt` would have written to the file.
//
// **A refusal is no edit at all.** `nvs fmt --stdin` writes nothing to standard output for a file
// it will not parse and exits non-zero (`crates/nvs-cli/src/fmt.rs`'s `stdin`), and a binary that
// is not there fails the same way; both leave every byte the user typed. Neither is reported from
// here: saving a file that does not parse yet is the ordinary state of one being written, and a
// binary that did not run is what `extension.ts`'s status item already says, for the server it
// could not start with it.
//
// **Every failure leaves less formatting, never a changed program.** A regions request nothing
// answers leaves the markup exactly as `nvs fmt` wrote it, a chunk the editor's formatter answers
// nothing for keeps the text it had, and an answer that would have moved a byte of a `<?= … ?>`
// hole is dropped whole rather than applied in part.
//
// What the edit is, given the buffer, what `nvs fmt` answered and where the markup in it is, is
// `template.ts`'s — the half that imports no `vscode`, and so the half the headless tier runs.

import { execFile } from "node:child_process";

import {
  DocumentFormattingEditProvider,
  ExtensionContext,
  Position,
  Range,
  TextDocument,
  TextEdit,
  commands,
  languages,
  workspace,
} from "vscode";

import { binary } from "./binary";
import { embedding, regions } from "./regions";
import { Chunk, chunks, edited, formatted, hidden, merged } from "./template";

// What this provider claims, which is `extension.ts`'s `SELECTOR`: a Novis file the editor has
// open from disk, and nothing else. A virtual document is not one — the markup shown to the HTML
// service is that service's to lay out.
const SELECTOR = [{ scheme: "file", language: "nvs" }];

// A formatted file is the same order of size as the one it came from, and Node's default cap on a
// child process's output is a megabyte, which a few thousand lines of Novis reaches. Nothing is
// held past the edit: the text crosses this module and goes back to the editor.
const OUTPUT_CEILING = 64 * 1024 * 1024;

// The setting that turns the markup pass off, default `true`. With it off a format request is
// `nvs fmt` and nothing else, which is what a user whose markup whitespace is output, or whose HTML
// tooling is their own, has to be able to ask for. The decision is `template.ts`'s `chunks`.
const SETTING = "nvs.template.format";

// The editor service whose formatter lays out a chunk, in the spelling `template.ts` maps the
// server's regions to.
const HTML = "html";

// What one level is, in `nvs fmt`'s four-space unit rather than the editor's `tabSize`: a chunk's
// nesting and the Novis around it have to agree on what a level is (ADR 0173 § 3).
const UNIT = { tabSize: 4, insertSpaces: true };

/**
 * Register the formatter, once, from `activate`.
 *
 * It is registered whether or not a server is running: `nvs fmt` is its own process and asks the
 * server nothing, so a file formats in a window where `nvs lsp` never started.
 */
export function install(context: ExtensionContext): void {
  context.subscriptions.push(
    languages.registerDocumentFormattingEditProvider(SELECTOR, provider),
  );
}

const provider: DocumentFormattingEditProvider = {
  async provideDocumentFormattingEdits(document: TextDocument): Promise<TextEdit[]> {
    const text = document.getText();
    const novis = await fmt(text);
    const pass = novis === undefined ? undefined : await markup(document, novis);
    const replacement = formatted(text, pass);
    if (replacement === undefined) {
      return [];
    }
    return [TextEdit.replace(whole(document, text), replacement)];
  },
};

/**
 * `novis` with every chunk of markup in it laid out by the editor's own HTML formatter.
 *
 * The regions are asked for over that text rather than over the buffer, since the buffer holds
 * neither the layout `nvs fmt` just chose nor, under format-on-save, anything saved yet. Nothing is
 * asked of the server at all for a pass the user turned off: the setting is read where the file is,
 * because a workspace folder with its own HTML tooling is exactly the one that turns it off.
 */
async function markup(document: TextDocument, novis: string): Promise<string> {
  const enabled = workspace.getConfiguration(undefined, document.uri).get<boolean>(SETTING, true);
  const answered = enabled ? await regions(document, novis) : [];
  const list = chunks(enabled, novis, answered, HTML);
  const laid: (string | undefined)[] = [];
  for (const chunk of list) {
    // One at a time rather than all at once: each ask puts its chunk in a virtual document of its
    // own, and a formatter answering about one of them is what says the next may go out.
    laid.push(await formatting(document, novis, chunk));
  }
  return merged(novis, list, laid);
}

/**
 * One chunk as the editor's HTML formatter lays it out, or nothing where it laid out none.
 *
 * The formatter is shown the chunk alone, with each hole standing in as text of its own length, so
 * what it answers about is markup and only markup. A range request rather than a document one
 * because a chunk is a range of a document this extension made up — and because
 * `vscode.executeFormatDocumentProvider` over anything in a `.nvs` file is the second formatter of
 * Novis the surfaces suite refuses.
 */
async function formatting(
  document: TextDocument,
  novis: string,
  chunk: Chunk,
): Promise<string | undefined> {
  const body = hidden(novis, chunk);
  if (body === undefined) {
    return undefined;
  }
  const uri = embedding(document, HTML, `chunk${chunk.start}`, body);
  try {
    const edits = await commands.executeCommand<TextEdit[] | undefined>(
      "vscode.executeFormatRangeProvider",
      uri,
      new Range(new Position(0, 0), ending(body)),
      UNIT,
    );
    return edits === undefined ? undefined : edited(body, edits);
  } catch {
    // A service that is not installed, or that threw over markup it could not parse: the chunk
    // keeps the text `nvs fmt` left it with, which is this pass's answer to everything it cannot do.
    return undefined;
  }
}

/** The last position of `text`, as the end of the range a chunk is formatted over. */
function ending(text: string): Position {
  const rows = text.split("\n");
  return new Position(rows.length - 1, rows[rows.length - 1].length);
}

/**
 * `nvs fmt --stdin` over `text`, or nothing where it refused.
 *
 * The exit status is what decides, and not whether anything was printed: an empty file formats to
 * an empty document and succeeds, where a refusal prints nothing and fails. A failure to spawn
 * arrives here as the same refusal, which is the direction that cannot damage a buffer.
 */
function fmt(text: string): Promise<string | undefined> {
  return new Promise((resolve) => {
    const child = execFile(
      binary(),
      ["fmt", "--stdin"],
      { maxBuffer: OUTPUT_CEILING },
      (failure, stdout) => resolve(failure === null ? stdout : undefined),
    );
    // A process that never started still has a pipe to write to, and writing to it raises an
    // error this promise has no use for — the callback above has already resolved to no edit.
    child.stdin?.on("error", () => undefined);
    child.stdin?.end(text);
  });
}

/** The whole of `document`, as the range one edit replaces. */
function whole(document: TextDocument, text: string): Range {
  return new Range(new Position(0, 0), document.positionAt(text.length));
}
