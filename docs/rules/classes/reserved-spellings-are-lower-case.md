Keywords are matched exactly: `if` is a keyword, `IF` and `If` are not. The same holds for the
contextual keywords — `spawn`, `script`, `with`, `type`, `from`, `by`, `get`, `set` — for the
`true`/`false`/`null` literals, and for the `<?nvs` open tag.

A mis-cased keyword gets no diagnostic of its own, deliberately. The casing rule makes `IF`, `ECHO` and
`TRUE` legal class names, so nothing lexical distinguishes a mistyped `if` from a deliberate reference
to a class called `IF`. The lexer emits an ordinary identifier and the program fails later as an
undefined name or a parse error. A heuristic would be the one place in the compiler that guesses, and
case-normalising reserved words is the easiest thing the converter does.

`<?nvs` is the one exception, because it is the one spelling that cannot be anything else: `<?NVS` is
recognised, reported, and still opens code mode — an unrecognised tag collapses the whole file into one
inline-HTML token and teaches the author nothing. Two things fall out: `Core\Bytes` is three ordinary
name tokens rather than a collision with the `bytes` type keyword, and identifier lexing no longer
allocates a lower-cased copy of every name in the file.
