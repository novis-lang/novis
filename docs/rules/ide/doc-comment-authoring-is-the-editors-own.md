Writing a `///` run is the editor's job, and it is two affordances that contribute no name.
`onEnterRules` continues the run — Enter on a line opening with `///` starts the next one at the same
indentation, and it keeps arriving until the author deletes it, as in Rust and C#. A
`DocumentPasteEditProvider` offers *Paste as doc comment* when multi-line text is pasted with the cursor
in a run: each line takes the run's marker and indentation, and an empty line becomes a bare `///`.

Both exist because `rule:tooling/doc-comment-is-three-slashes` is line-oriented on purpose — the per-line
marker is what stops anything in the body from ending the comment — and the cost of that is authoring,
not reading. VS Code's toggle-line-comment inserts `//`, so it answers neither half.

Neither is a command. The editor already surfaces paste alternatives in its own widget
(`rule:ide/the-extension-builds-no-ui-the-editor-already-has`), so nothing is added to a menu or a
keymap, and `rule:ide/contributions-are-frozen-and-only-ever-added`'s roster does not move — except that
the paste edit's **kind id** reaches a user's `editor.pasteAs.preferences` and is frozen on that rule's
own terms. Prefixing lines with a marker is not language logic, so
`rule:ide/dependencies-are-allowlisted`'s allowlist is unchanged; an aid that had to understand the prose
inside a doc comment is the first thing here that would have to answer it.

Each client owns its own copy (`rule:ide/one-server-two-thin-clients`): these are editing affordances,
not server answers, so another editor client would write both again or go without.

No rule guesses where a doc comment ends. The continuation over-runs by one line until the author deletes
it, the familiar nuisance of every `///` language, and a heuristic would guess wrong in the other
direction. Nothing checks what the paste provider writes, but prefixing lines with a marker cannot
produce anything that is not a comment, so the worst case is a badly worded one.

- **Memory:** none. Both affordances run in the editor and touch no compile and no request.

**Built:** the `onEnterRules` continuation, in `editors/vscode/language-configuration.json`.
**Missing:** *Paste as doc comment*. The extension's one `DocumentPasteEditProvider` is the import
carried with a pasted type (`editors/vscode/src/imports.ts`).

**Declined:**
- **A `///`-delimited block** — `///` alone opening, an un-prefixed body, `///` alone closing. Its
  terminator is a bare `///` line, which is what a code sample of a documented member contains, so ending
  it safely needs a lexer that tracks Markdown fences. It makes the body's indentation content (four
  spaces is a CommonMark code block) while `rule:tooling/fmt-quotes` forbids re-aligning a comment, and
  an unterminated one turns the rest of the file into prose.
- **`/** */`, the PHPDoc shape.** `rule:tooling/doc-comment-is-three-slashes` declines it, and a block
  comment ends at the first `*/`, so a glob or a regex in the Markdown body would end the comment.
- **A command and a keybinding** for the paste. They add a frozen id and a key to learn for what the
  paste widget already offers.
- **A snippet that expands to a doc comment.** A snippet body is a second copy of a syntactic shape
  (`rule:ide/a-code-action-writes-only-what-is-already-determined`).
- **`nvs fmt` inserting the prefix.** `rule:tooling/fmt-quotes` leaves every comment byte-for-byte
  untouched, and a formatter cannot know which un-prefixed lines were meant as documentation.
