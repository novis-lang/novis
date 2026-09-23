- **The extension host tier cannot drive a copy, so no `prepareDocumentPaste` ever runs in it.**
  `editor.action.clipboardCopyAction` fills the clipboard and consults no copy provider — neither the
  extension's nor a second one the case registers — while `provideDocumentPasteEdits` *is* consulted
  on a paste, so a round-trip case pastes the right text and gains none of the edit meant to ride
  with it. Assert the paste half with a provider the case registers under the extension's own kind,
  the way `editors/vscode/test/host/imports.test.ts` does. [until: reviewed 2026-09-22]
