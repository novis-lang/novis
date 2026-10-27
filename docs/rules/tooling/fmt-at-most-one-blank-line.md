No two blank lines follow each other in code, and no blank line sits right after a line that ends in
an opening `{`, `(` or `[`, or right before a line that starts with its closing `}`, `)` or `]`. A run
of several blank lines becomes one, and a blank line at the inside edge of a body, a list or a `match`
arm list is deleted.

Every blank line `rule:tooling/fmt-base-style-is-per` requires is still exactly one, and every other
blank line it leaves to the author still stands, only never more than one in a row. A blank line
between two statements or two simple properties is how an author groups them, so it is kept.

Bytes a program prints and the text of a comment are not code. A blank line inside a heredoc or nowdoc
body, an html template's body, an inline-HTML region or a block comment is copied as it is. A file ends
with exactly one newline, as `rule:tooling/fmt-base-style-is-per` says.

What it buys is the same as `rule:tooling/fmt-one-run-between-two-tokens`: two files that differ only in
how many empty lines they have format to the same bytes. A blank line right after an opening brace
groups nothing, so it is deleted rather than kept.

**Declined:**
- **Keeping every blank line.** Three blank lines and one would stay two different files.
- **Deleting every blank line inside a body.** It deletes how the author grouped the statements.
- **Two blank lines between top-level declarations, as Python's style allows.** It is a second count
  for the reader to learn.
