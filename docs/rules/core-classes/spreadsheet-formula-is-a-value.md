Only an explicit `Formula` value produces a formula cell. Every string writes as a **text cell** —
including one beginning `=`, `+`, `-`, `@` or a control character, which is the whole trigger set for
the formula-injection class.

An export built from user data is therefore inert by construction rather than by a caller remembering
to prefix a quote. This is the same move taint tracking makes everywhere else — code and data
separated by type, not by inspection — applied at a boundary that is otherwise a footnote in a
security advisory.

What it costs is one wrapper at the call sites that genuinely mean a formula, which is the smaller
half of any real export. Because the writer cannot be tricked into emitting code, it needs no
launderer, which is part of why it can live outside `Core`.

The component's fixtures owe a `tainted` string beginning `=` that lands as a text cell, and the same
expression as a `Formula` value that lands as a formula cell, both asserted on the written bytes
rather than the builder's state.

**Not shipped.** There is no spreadsheet package in the tree.
