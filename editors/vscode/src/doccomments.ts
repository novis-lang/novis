// The editor side of *Paste as doc comment*: a paste provider that asks `docpaste.ts` and offers what
// it answers (`rule:ide/doc-comment-authoring-is-the-editors-own`).
//
// The edit is offered, never applied on its own: no preference this extension contributes names its
// kind, so the editor pastes the plain text and lists this edit in its own paste widget. The
// extension adds no command and no menu entry for it (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
// It needs no server, so it works before one answers and with none installed.

import {
  CancellationToken,
  DataTransfer,
  DocumentDropOrPasteEditKind,
  DocumentPasteEdit,
  DocumentPasteEditContext,
  DocumentPasteEditProvider,
  EndOfLine,
  ExtensionContext,
  languages,
  Range,
  TextDocument,
} from "vscode";

import { DOC_COMMENT_KIND, DOC_COMMENT_TITLE, asDocComment } from "./docpaste";

const KIND = DocumentDropOrPasteEditKind.Empty.append(...DOC_COMMENT_KIND.split("."));

/** Register the paste provider. Called once from `activate`. */
export function install(context: ExtensionContext): void {
  context.subscriptions.push(
    languages.registerDocumentPasteEditProvider({ language: "nvs" }, new Provider(), {
      providedPasteEditKinds: [KIND],
      pasteMimeTypes: ["text/plain"],
    }),
  );
}

class Provider implements DocumentPasteEditProvider<DocumentPasteEdit> {
  /**
   * The paste with each later line written as a line of the run, or `undefined` where it is not
   * offered. A paste at more than one cursor is not offered, because each cursor's run may have its
   * own indentation and an edit has one text.
   */
  async provideDocumentPasteEdits(
    document: TextDocument,
    ranges: readonly Range[],
    dataTransfer: DataTransfer,
    _context: DocumentPasteEditContext,
    token: CancellationToken,
  ): Promise<DocumentPasteEdit[] | undefined> {
    if (ranges.length !== 1) {
      return undefined;
    }
    const text = await dataTransfer.get("text/plain")?.asString();
    if (text === undefined || token.isCancellationRequested) {
      return undefined;
    }
    const start = ranges[0].start;
    const before = document.lineAt(start.line).text.slice(0, start.character);
    const eol = document.eol === EndOfLine.CRLF ? "\r\n" : "\n";
    const written = asDocComment(before, text, eol);
    if (written === undefined) {
      return undefined;
    }
    return [new DocumentPasteEdit(written, DOC_COMMENT_TITLE, KIND)];
  }
}
