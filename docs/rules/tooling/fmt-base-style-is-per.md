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
members of a class, interface or enum that each have a body: a method with a block, a property with
hooks, or an enum. A comment between two members belongs to the member below it, so the blank line goes
above the comment, and a comment that ends a member's last line stays on that line, above the blank line.
Every other blank line is the author's: between two members when either has no body, inside the `use`
block, and between a comment and the member it describes. A blank line between two simple properties
groups them, PER fixes no grouping, and a formatter that deleted it would delete what the author meant.
The author's blank lines are never more than one in a row, and none sits at the inside edge of a body
(`rule:tooling/fmt-at-most-one-blank-line`).

A property's hooks sit one level in from the property's line, and a hook's block is a body of its own:
its statements sit one level in from the hook's line, and its `}` on the hook's.

A `switch` is laid out as PER-CS lays it out: each `case` and `default` sits one level in from the
`switch` line, a case's statements one level in from that case's line, and the closing brace on the
`switch` line. A template region's own text, every byte between a `?>` and the next open tag, is what
the program prints, so its lines are copied exactly as written and never re-indented.

A comment that opens a line sits at the indentation of the code below it, which is the line it describes.
A comment above a body's closing brace is inside that body, so it sits where the body's statements sit.
A line of a block comment after its first is the comment's own text and is copied.

Modifier order is canonical, not the author's: `abstract`/`final`, then visibility (including
`private(set)`), then `static`, then `readonly`, then `lateinit`, one space between each. The parser
accepts these in any order, which is exactly why the formatter must fix one — `static public $x;` and
`public static $x;` both compile and would otherwise never converge. A missing modifier is never
supplied: a formatter that changes meaning is not a formatter (`rule:core-api/written-visibility`).
