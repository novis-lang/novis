A `secret` value may not become output, a log line, a dump or a `Throwable` message
(`rule:security/secret-sinks-refuse`), and the toolchain is held to the bar it enforces on everyone
else: no command this project ships prints one in the clear. The diagnostic record already carries
the redaction as a node kind, which is what makes plaintext, JSON and HTML renderings agree from one
place (`rule:errors/record-transformations`).

The AST dump carries **no literal text**: a literal node has its span, so a reader holding the file
already has the text, and a reader without the file is never handed a `secret` literal's bytes
(`crates/nvs-syntax/src/walk.rs`, `rule:ide/ast-json-schema-is-frozen`). The dump needs no placeholder,
because the parse that writes it cannot know which literal is `secret` and so prints none of them.

A compiler diagnostic's source snippet is the surface still missing. It quotes the lines a diagnostic
points at as written, a `secret` literal among them, because a compiler diagnostic is not a producer
of the diagnostic record whose redaction node the other renderings share
(`crates/nvs-diagnostics/src/render.rs`, `data/gaps/nvs-render/compiler-diagnostic-is-not-a-producer.json`).

This rule is about what the toolchain **prints**, and it is enforced by the compiler for every caller.
What an editor draws over a buffer it did not print is a separate mechanism in a separate chapter
(`rule:ide/redaction-ranges-come-from-the-server`), and neither one covers for the other: the
dump holds no literal with no editor running, and no decoration anywhere puts a value back into a
rendering this rule has already redacted.

**Partly on disk.** The dump and every rendering of the diagnostic record print no `secret` in the
clear. A compiler diagnostic's snippet does.
