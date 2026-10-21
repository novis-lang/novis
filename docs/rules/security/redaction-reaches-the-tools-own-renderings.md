A `secret` value may not become output, a log line, a dump or a `Throwable` message
(`rule:security/secret-sinks-refuse`), and the toolchain is held to the bar it enforces on everyone
else: no command this project ships prints one in the clear. The diagnostic record already carries
the redaction as a node kind, which is what makes plaintext, JSON and HTML renderings agree from one
place (`rule:errors/record-transformations`).

The AST dump carries **no literal text**: a literal node has its span, so a reader holding the file
already has the text, and a reader without the file is never handed a `secret` literal's bytes
(`crates/nvs-syntax/src/walk.rs`, `rule:ide/ast-json-schema-is-frozen`). The dump needs no placeholder,
because the parse that writes it cannot know which literal is `secret` and so prints none of them.

A compiler diagnostic's source snippet prints a `secret` literal **masked**: one `*` per character
between its quotes, so the caret under a later argument still points at it. Which literals are
`secret` is the checker's answer, from the walk the editor's redaction ranges come from
(`crates/nvs-types/src/redaction.rs`), so the terminal and the editor conceal the same bytes. The
compile path hands those ranges to the source map, and the diagnostic's record takes its snippet
line from there (`crates/nvs-diagnostics/src/record.rs`), so the text and `nvs check --json` both
inherit it. A suggestion whose edit touches a concealed literal is left out of the record, because
its replacement could quote it; `nvs check --fix` still applies it. A file that does not parse has
no checker answer, so a snippet under a syntax error is printed as written.

The mask keeps the literal's length visible. That is the trade an editor's decoration already makes
(`rule:ide/redaction-covers-bytes-only`), and it buys a caret that stays under the code it names.

This rule is about what the toolchain **prints**, and it is enforced by the compiler for every caller.
What an editor draws over a buffer it did not print is a separate mechanism in a separate chapter
(`rule:ide/redaction-ranges-come-from-the-server`), and neither one covers for the other: the
dump holds no literal with no editor running, and no decoration anywhere puts a value back into a
rendering this rule has already redacted.

**Declined:**
- **`[redacted]` in place of the literal.** It hides the length, and it moves every character after
  it on the line, so a caret drawn from the record's columns lands under the wrong code.
