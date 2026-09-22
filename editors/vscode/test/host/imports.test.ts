// The editor's half of paste-with-imports (`rule:ide/a-pasted-type-carries-its-use-line`), asserted in
// a real one.
//
// The feature is two asks of the server at two moments: one when text is copied out of a Novis document,
// one when it is pasted into another. What each answers is `crates/nvs-lsp/tests/imports.rs`, and both
// over the wire are `test/protocol/requests.test.ts`. What neither can see is the editor *applying* the
// edit rather than offering it in a widget. It applies a provider's paste edit on its own only for a
// kind its `editor.pasteAs.preferences` names, that list is empty by default, and `package.json`
// contributes `["text.updateImports"]` as the default for Novis documents. While that contribution was
// missing, every other test of this feature stayed green and a user pasting into a `.nvs` file got plain
// text — so this is the tier that would have caught it, and the only one.
//
// The copy half cannot be driven from here, and `src/imports.ts` records that gap:
// `editor.action.clipboardCopyAction` puts the text on the clipboard and runs no `prepareDocumentPaste`
// at all — not this extension's, and not one this file registers itself — so nothing ever puts the
// extension's own MIME type on the clipboard and the round trip has no editor-level proof. The provider
// registered below therefore stands in for `src/imports.ts`, under the same kind: what is asserted is
// that this editor applies an edit offered under that kind with no pick, which is the contract the
// extension's own provider is built on.
//
// This file runs after `colour.test.ts` — Mocha loads the directory sorted — whose third test turns
// `nvs.lsp.enable` on. Nothing here needs a server, and nothing here stops one.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";

import { shown, until } from "./editor";

/** The kind `src/imports.ts` offers its edit under, and the kind the contributed preference names. */
const KIND = vscode.DocumentDropOrPasteEditKind.TextUpdateImports.append("nvs");

/** What the stand-in provider writes at the top of the document when the editor applies its edit. */
const MARK = "// the paste edit was applied\n";

/** The text the clipboard carries, which is what an ordinary paste would put there on its own. */
const PASTED = "echo 'pasted';";

/** What `editor.pasteAs.preferences` resolves to for one language, as this editor reads it. */
function preferences(languageId: string): unknown {
  return vscode.workspace.getConfiguration("editor", { languageId }).get("pasteAs.preferences");
}

describe("a paste into a Novis document", () => {
  it("applies an edit that updates imports, with no pick", async () => {
    // The setting nobody wrote. It is contributed under `[nvs]`, so a language that is not Novis reads
    // the editor's own empty default and a paste there is offered rather than applied — which is what
    // the two readings together say, and neither says on its own.
    assert.deepEqual(preferences("nvs"), ["text.updateImports"]);
    assert.deepEqual(preferences("plaintext"), [],
                     "the preference is set for every language, so it says nothing about Novis");

    const editor = await shown("hello.nvs");
    const offered = vscode.languages.registerDocumentPasteEditProvider({ language: "nvs" }, {
      async provideDocumentPasteEdits(
        document: vscode.TextDocument,
        _ranges: readonly vscode.Range[],
        dataTransfer: vscode.DataTransfer,
      ): Promise<vscode.DocumentPasteEdit[]> {
        const text = (await dataTransfer.get("text/plain")?.asString()) ?? "";
        const edit = new vscode.DocumentPasteEdit(text, "Paste and add the missing `use` lines", KIND);
        const additional = new vscode.WorkspaceEdit();
        additional.insert(document.uri, new vscode.Position(0, 0), MARK);
        edit.additionalEdit = additional;
        return [edit];
      },
    }, { providedPasteEditKinds: [KIND], pasteMimeTypes: ["text/plain"] });

    try {
      await vscode.env.clipboard.writeText(PASTED);
      const end = editor.document.lineAt(editor.document.lineCount - 1).range.end;
      editor.selection = new vscode.Selection(end, end);
      await vscode.commands.executeCommand("editor.action.clipboardPasteAction");

      // Both halves, in one wait. The text alone arriving is the ordinary paste, which is exactly what
      // an empty preference list leaves behind, so waiting for the text would pass on the defect this
      // test is here for.
      await until("the pasted text arriving with the provider's edit applied", async () => {
        const now = editor.document.getText();
        return now.includes(PASTED) && now.startsWith(MARK) ? true : undefined;
      }, () => {
        const now = editor.document.getText();
        return now.includes(PASTED)
          ? "the text was pasted and the edit offered with it was not applied"
          : "nothing was pasted";
      });
    } finally {
      offered.dispose();
      // The fixture copy outlives this test — `surfaces.test.ts` runs after it — and the paste was
      // never saved, so reverting is what leaves the workspace the way the launcher wrote it.
      await vscode.commands.executeCommand("workbench.action.files.revert");
    }
  });
});
