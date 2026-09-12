// Formatting a `.nvs` file: `nvs fmt` over the buffer, and one edit back.
//
// The provider registered here is the whole of the client's part in it, and it lays out nothing
// (`rule:ide/one-server-two-thin-clients`). `nvs fmt` is the only formatter of Novis there is and
// its layout is unconfigurable by decision (`rule:tooling/fmt-is-one-canonical-style`), so what a
// format request in a Novis file means is that process run over the buffer — not a second set of
// rules living in an editor, which is the drift `rule:ide/one-server-two-thin-clients` exists to
// stop.
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
// What the edit is, given the buffer and what the formatter answered, is `template.ts`'s
// `formatted` — the half that imports no `vscode`, and so the half the headless tier runs.

import { execFile } from "node:child_process";

import {
  DocumentFormattingEditProvider,
  ExtensionContext,
  Position,
  Range,
  TextDocument,
  TextEdit,
  languages,
} from "vscode";

import { binary } from "./binary";
import { formatted } from "./template";

// What this provider claims, which is `extension.ts`'s `SELECTOR`: a Novis file the editor has
// open from disk, and nothing else. A virtual document is not one — the markup shown to the HTML
// service is that service's to lay out.
const SELECTOR = [{ scheme: "file", language: "nvs" }];

// A formatted file is the same order of size as the one it came from, and Node's default cap on a
// child process's output is a megabyte, which a few thousand lines of Novis reaches. Nothing is
// held past the edit: the text crosses this module and goes back to the editor.
const OUTPUT_CEILING = 64 * 1024 * 1024;

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
    const replacement = formatted(text, await fmt(text));
    if (replacement === undefined) {
      return [];
    }
    return [TextEdit.replace(whole(document, text), replacement)];
  },
};

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
