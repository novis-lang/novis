Four-space indentation and no tabs; one statement per line; a file ends with exactly one newline and no
line carries trailing whitespace.

Braces split by *kind of body*. Every control structure — `if`/`elseif`/`else`, `while`, `do`/`while`,
`for`, `foreach`, `switch`, `try`/`catch`/`finally` — is K&R: the opening brace stays on the keyword's
line after one space, the closing brace starts its own line, and `elseif`, `else`, `catch` and `finally`
continue on the closing brace's line. `elseif` is one word, never `else if`. Every declaration with a
body — `class`, `interface`, `enum`, a named function or method, including an interface method that
carries a body — is Allman: the opening brace starts its own line at the declaration's indentation.
The one exception is an enum whose cases are on one line, which keeps its `{` on the `enum` line
(`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line`).

Exactly one blank line follows a `namespace` line, one follows the `use` block, and one separates two
members that each have a body; adjacent simple property and constant declarations get none. A comment
between two members belongs to the member below it, so the blank line goes above the comment.

Modifier order is canonical, not the author's: `abstract`/`final`, then visibility (including
`private(set)`), then `static`, then `readonly`, then `lateinit`, one space between each. The parser
accepts these in any order, which is exactly why the formatter must fix one — `static public $x;` and
`public static $x;` both compile and would otherwise never converge. A missing modifier is never
supplied: a formatter that changes meaning is not a formatter (`rule:core-api/written-visibility`).
