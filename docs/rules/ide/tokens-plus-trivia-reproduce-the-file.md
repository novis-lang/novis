Concatenating every token's and every trivium's source text, in offset order, must equal the file byte
for byte. Losslessness is a property that is tested, not asserted: one test over the whole corpus —
`examples/`, `tests/`, and the source inside every `.nvst` case — proves that no byte of a source file
disappears between the lexer and the tree.

`Trivia { kind, span }` comes from one edit: `Lexer` gains a flag and `skip_trivia` pushes a trivium
instead of only advancing. `TriviaKind` is `Whitespace`, `LineComment` (`//`, `#`, and a run of four or
more slashes), `BlockComment`, or `DocComment` — exactly three slashes, the only variant anything but a
formatter reads. Nothing else in the lexer changes, because nothing else discards a byte.

This is the prerequisite `nvs fmt` rests on: a formatter that promises comments survive it cannot keep
that promise over a stream that drops them.
