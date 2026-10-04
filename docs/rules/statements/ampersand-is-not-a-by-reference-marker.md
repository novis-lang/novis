`&` where a by-reference marker would go is still recognised, so the diagnostic can name the exact fix, and
produces an error node — the AST keeps no variant for it. `E0237`: *`&` is not a by-reference marker in
Novis — write `inout` before the type.*

`&` keeps its other three jobs unchanged: `$a & $b` is bitwise AND, `$a && $b` is logical AND, and `A&B` is
an intersection type. The one-token lookahead that told `int &$x` apart from an intersection continuation
survives the removal of the meaning it was written for, because meaning it and parsing it are two different
questions: the type parser reaches that `&` first, and consuming it as an intersection member would leave
the site above it nothing to report but a malformed intersection.

The already-refused spellings keep their own codes and gain the new word in their help text: `$a = &$b` is
`E0701`, an anonymous function parameter `E0493`, a generator's `E0492`.
