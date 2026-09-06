Every rendering the toolchain itself produces inherits the redaction, so no surface prints the
plaintext the editor is concealing. The diagnostic record already carries it as a node kind, which is
what makes plaintext, JSON and HTML renderings agree from one place
(`rule:errors/record-transformations`).

The AST dump is the surface that would otherwise disagree: a node's own scalar fields include a string
literal's text, so a panel reading it would render a secret the buffer behind it is blurring. A
literal node whose static type carries `secret` emits the same fixed placeholder a dumped property
gets, **in the JSON itself** rather than in the viewer, so the command-line output and the panel
cannot diverge.

The cost is that a frozen dump schema now has a type-dependent field value, and a reader can no longer
assume a literal node's text is the source text.

**Not on disk.** There is no language server, and the dump emits no such placeholder.
