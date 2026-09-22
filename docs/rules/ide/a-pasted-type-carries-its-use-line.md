A type name pasted from one `.nvs` file into another brings the `use` line that makes it resolve there,
read at copy time from the file it came from. Two requests of Novis's own carry it: `nvs/imports`, asked
in the source document when text is copied, answers every type name the copied range wrote — the receiver
of a `::` access, the class after `new`, every name in an annotation — paired with what it resolved to
through that file's imports and namespace, and the editor keeps the answer on the clipboard beside the
text under a MIME type of the extension's own. `nvs/importEdits`, asked in the destination when that
clipboard is pasted, takes those pairs and the paste position and answers the `use` lines the destination
lacks, as one edit at the place every `use` line this server writes goes
(`rule:ide/a-bare-name-reaches-every-type-and-imports-the-one-accepted`'s place, `nvs_hir::imports`).

**What is carried is what the source resolved, never what the text looks like.** A name is put through
the same lookup a click follows, with the namespace and the imports in force at the position it was
written; one that resolves to nothing declared is not carried, because importing it fixes nothing, and a
qualified name is carried as written and imported nowhere, because it is absolute
(`rule:statements/a-qualified-name-is-absolute`). At the destination a name is imported only where it
would not resolve as it was written — a namespace that already reaches it, or a `use` already in force,
means no line — and only under a short name nothing there already answers to; a name that cannot be
imported is left as the paste wrote it, and the checker's own diagnostic on it offers the fix
(`rule:ide/an-undeclared-name-offers-its-import`).

**The paste is the editor's, not a command.** The edit is offered under the `text.updateImports` kind
the editor's own `editor.pasteAs.preferences` prefers by default, so the import arrives with the paste
and nothing is added to a menu, a keymap or the settings
(`rule:ide/the-extension-builds-no-ui-the-editor-already-has`); a user who wants plain text back takes
the kind out of that list. A paste that needs no import, a clipboard that carries no answer of the
extension's, and a server that is not running are all the editor's ordinary paste. The copy-time ask is
what makes a paste into another window, or after the source was closed or edited, still correct: what
the text meant is a fact about the file as it was at the copy. Each client owns its own copy of the two
asks (`rule:ide/one-server-two-thin-clients`).

What it spends is one analysis of the source at copy time and one of the destination at paste time, both
of documents the server already holds, and nothing between the two. The two answers have no `.lspt`
spelling — a list of name pairs and a text edit — and are held by a Rust test on the wire shapes instead
(`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`).
