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
not server answers, so the PhpStorm plugin writes both again or goes without.
