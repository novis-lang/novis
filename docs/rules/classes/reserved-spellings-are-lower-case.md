Keywords are matched exactly: `if` is a keyword, `IF` and `If` are not. The same holds for the
contextual keywords — `spawn`, `script`, `with`, `await`, `type`, `from`, `by`, `get`, `set`,
`discover` — for the `true`/`false`/`null` literals, and for the `<?nvs` open tag.

A contextual keyword lexes as an ordinary identifier and is read as a keyword only where what follows
it fits its production: `script` after `spawn`, an operand after `await`, `discover` directly after
`autoload`. So a program that already uses one of these spellings as a constant, a function, a method or
a class name still parses. The one reading this claims is `await($x)`, which is the operator over a
parenthesised operand and not a call to a function named `await`.

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
