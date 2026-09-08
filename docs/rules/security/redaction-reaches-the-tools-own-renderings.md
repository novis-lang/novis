A `secret` value may not become output, a log line, a dump or a `Throwable` message
(`rule:security/secret-sinks-refuse`), and the toolchain is held to the bar it enforces on everyone
else: no command this project ships prints one in the clear. The diagnostic record already carries
the redaction as a node kind, which is what makes plaintext, JSON and HTML renderings agree from one
place (`rule:errors/record-transformations`).

The AST dump is the surface that would otherwise disagree: a node's own scalar fields include a string
literal's text, so anything reading the dump would render a value every other rendering redacts. A
literal node whose static type carries `secret` emits the same fixed placeholder a dumped property
gets, **in the JSON itself** rather than in whatever displays it, so two readers of one dump cannot
diverge.

The cost is that a frozen dump schema now has a type-dependent field value, and a reader can no longer
assume a literal node's text is the source text.

This rule is about what the toolchain **prints**, and it is enforced by the compiler for every caller.
What an editor draws over a buffer it did not print is a separate mechanism in a separate chapter
(`rule:ide/redaction-ranges-come-from-the-server`), and neither one covers for the other: the
placeholder here holds with no editor running, and no decoration anywhere puts a value back into a
dump this rule has already redacted.

**Not on disk.** The dump emits no such placeholder.
