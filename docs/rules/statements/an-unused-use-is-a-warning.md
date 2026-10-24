A `use` line that no name in its namespace scope reads is the warning `W1026`. `nvs check` reports it
for every file of the program outside a `vendor` directory, and the editor shows it on the open document
with the `Unnecessary` tag, which fades the line. It is the one entry an unused import gets: the editor
sends no separate dimming hint for it. An import whose path resolves to nothing has its own error and
no warning.

**A construct the language does not allow is always an error, and a warning is for code that runs as
written.** An unused import changes nothing a program does, so it never fails a compile, and `nvs run`
and `nvs serve` print nothing about it. Only `nvs check` and the editor report it.

The warning carries one safe edit that deletes the statement, and the whole line when nothing but blank
space or a `//` comment shares it, so `nvs check --fix` removes the line. A deprecated use that `--fix`
rewrites can leave an import unused, and the next pass of the same run deletes it. `nvs fmt` keeps the
line, since it is layout only and has no name resolution (`rule:tooling/fmt-is-never-a-diagnostic`).

**The source is the editor's reference index.** A name reads an import when it is an identifier spelled
as the import's short name, in the same namespace scope, outside every `use` statement and not one
segment of a qualified name. `crates/nvs-lsp/src/index.rs`'s `reads` is that reading, the index keeps it
for organize imports, and `nvs_lsp::diagnostics::unused_uses` turns it into the warning for both
`nvs check` and the editor, so the two cannot disagree about a line. The reading errs one way only: a
method or a property spelled like the import counts as a read, so an unused import can go unreported,
and a needed one is never deleted.

What it costs is one more lexing of each file `nvs check` reads, and of the open document per analysis.
Nothing runs on the request path.

**Declined:**
- **The name resolution of `nvs-hir` as the source.** It would report every import a name resolves
  through, which is more exact, but a type annotation resolves in the type phase, so the warning would
  need a second record of reads across two phases. The editor's dimming would then read one source and
  `nvs check` another.
- **The warning on every compile.** `nvs run` and the test runner would print it beside the program's
  own output, and a case that freezes standard error would change with every unused import.
