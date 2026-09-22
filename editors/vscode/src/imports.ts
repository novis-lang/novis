// A type name pasted from one `.nvs` file into another brings its `use` line with it
// (`rule:ide/a-pasted-type-carries-its-use-line`).
//
// Two asks of the server, at two moments. When text is copied out of a Novis document the editor
// calls `prepareDocumentPaste`, and this file asks `nvs/imports` what type names the copied range
// wrote and what each resolved to *there* — through that file's `use` lines and namespace — and puts
// the answer on the clipboard beside the text, under a MIME type of its own. When that clipboard is
// pasted into a Novis document the editor calls `provideDocumentPasteEdits`, and this file asks
// `nvs/importEdits` which of those names would not resolve as written at the destination, gets the
// `use` lines back as edits, and hands the editor the paste with those edits attached. A paste that
// needs none is not answered at all, so the editor pastes the text the way it always did.
//
// The clipboard carries the imports rather than the paste re-reading the source, because by the
// time somebody pastes, the source may be closed, edited or in another window; what the copied text
// meant is a fact about the file as it was at the copy. It is also what lets a paste into a second
// VS Code instance still get its imports.
//
// The edit kind sits under `text.updateImports`. The editor applies a provider's paste edit on its own
// only for a kind its `editor.pasteAs.preferences` names, and that list is empty by default, so
// `package.json` contributes `["text.updateImports"]` as the default for Novis documents: the import
// then arrives with the paste, with no widget, command or setting of this extension's own
// (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). A user who wants plain text back
// overrides that preference in their own settings; nothing here adds a switch beside it. The kind id,
// the MIME type and the contributed default reach a user's settings and other extensions' providers,
// so all three are frozen on `rule:ide/contributions-are-frozen-and-only-ever-added`'s terms.

import {
  CancellationToken,
  DataTransfer,
  DataTransferItem,
  DocumentDropOrPasteEditKind,
  DocumentPasteEdit,
  DocumentPasteEditContext,
  DocumentPasteEditProvider,
  ExtensionContext,
  languages,
  Position,
  Range,
  TextDocument,
  WorkspaceEdit,
} from "vscode";
import { LanguageClient } from "vscode-languageclient/node";

// The server's two requests, spelled where `crates/nvs-lsp/src/imports.rs` spells them.
const METHOD = "nvs/imports";
const EDITS_METHOD = "nvs/importEdits";

// What the copy leaves on the clipboard beside the text: the server's `nvs/imports` answer, as JSON.
// A MIME type of this extension's own, because a paste must only trust an answer this extension put
// there — anything else on the clipboard is somebody else's.
const MIME = "application/vnd.code.nvs.imports";

// The kind the paste edit is offered under. `text.updateImports` is what the contributed preference
// names; the last segment is this language's, the way the TypeScript extension's is `jsts`.
const KIND = DocumentDropOrPasteEditKind.TextUpdateImports.append("nvs");

// The documents this provider is offered for, which is the language and nothing about the scheme:
// a copy out of a read-only preview still knows what it imported.
const SELECTOR = { language: "nvs" };

/** One type name the copied text wrote, and what it resolved to where it was copied from. */
interface Import {
  symbol: string;
  written: string;
}

/** One insertion the server answers, in LSP's own shape. */
interface TextEdit {
  range: { start: { line: number; character: number }; end: { line: number; character: number } };
  newText: string;
}

let serving: LanguageClient | undefined;

/**
 * Register the paste provider. Called once from `activate`, before any server is running: a copy
 * made while no server answers carries no imports and the paste is plain text, which is the same
 * paste the editor made before this file existed.
 */
export function install(context: ExtensionContext): void {
  context.subscriptions.push(
    languages.registerDocumentPasteEditProvider(SELECTOR, new Provider(), {
      providedPasteEditKinds: [KIND],
      copyMimeTypes: [MIME],
      pasteMimeTypes: [MIME],
    }),
  );
}

/** The client every ask goes to, or `undefined` while none is running. */
export function serve(client: LanguageClient | undefined): void {
  serving = client;
}

class Provider implements DocumentPasteEditProvider<DocumentPasteEdit> {
  /** At copy time: ask what the copied ranges import, and leave the answer on the clipboard. */
  async prepareDocumentPaste(
    document: TextDocument,
    ranges: readonly Range[],
    dataTransfer: DataTransfer,
    token: CancellationToken,
  ): Promise<void> {
    const client = serving;
    if (client === undefined) {
      return;
    }
    const carried: Import[] = [];
    for (const range of ranges) {
      let answered: Import[];
      try {
        answered = await client.sendRequest<Import[]>(
          METHOD,
          { textDocument: { uri: document.uri.toString() }, range: client.code2ProtocolConverter.asRange(range) },
          token,
        );
      } catch {
        // A copy is never held up by a server that cannot answer: the text still goes to the
        // clipboard, and the paste is plain text.
        return;
      }
      carried.push(...answered);
    }
    if (carried.length > 0 && !token.isCancellationRequested) {
      dataTransfer.set(MIME, new DataTransferItem(JSON.stringify(carried)));
    }
  }

  /**
   * At paste time: ask which of the carried names the destination does not resolve, and hand the
   * editor the paste with their `use` lines attached. `undefined` where the clipboard carries no
   * answer of ours, where the server is not running, or where nothing needs importing — every one
   * of those is the editor's ordinary paste.
   */
  async provideDocumentPasteEdits(
    document: TextDocument,
    ranges: readonly Range[],
    dataTransfer: DataTransfer,
    _context: DocumentPasteEditContext,
    token: CancellationToken,
  ): Promise<DocumentPasteEdit[] | undefined> {
    const client = serving;
    const item = dataTransfer.get(MIME);
    const first = ranges[0];
    if (client === undefined || item === undefined || first === undefined) {
      return undefined;
    }
    const text = await dataTransfer.get("text/plain")?.asString();
    if (text === undefined) {
      return undefined;
    }
    let imports: Import[];
    try {
      imports = JSON.parse(await item.asString()) as Import[];
    } catch {
      return undefined;
    }
    let edits: TextEdit[];
    try {
      edits = await client.sendRequest<TextEdit[]>(
        EDITS_METHOD,
        {
          textDocument: { uri: document.uri.toString() },
          position: client.code2ProtocolConverter.asPosition(first.start),
          imports,
        },
        token,
      );
    } catch {
      return undefined;
    }
    if (edits.length === 0 || token.isCancellationRequested) {
      return undefined;
    }
    const edit = new DocumentPasteEdit(text, "Paste and add the missing `use` lines", KIND);
    const additional = new WorkspaceEdit();
    for (const insertion of edits) {
      additional.insert(
        document.uri,
        new Position(insertion.range.start.line, insertion.range.start.character),
        insertion.newText,
      );
    }
    edit.additionalEdit = additional;
    return [edit];
  }
}
