Every whitespace run between two tokens on one line is decided by what the two tokens are, and it is
either nothing or exactly one space. A run that holds a line break is not this rule's: the line breaks
are the author's (`rule:tooling/fmt-never-reflows`) and the indentation after one is
`rule:tooling/fmt-base-style-is-per`'s. Where `nvs fmt` already decides a run in another rule — the
space before a K&R brace, a qualifier's space, the inside of a one-line anonymous object — that rule's
answer stands and agrees with this table.

A token's role comes from the tree, never from its spelling alone: the innermost syntax node holding a
`-` says whether it is `Sub` or `Neg`, and a `<`, `|`, `&` or `?` inside a written type's span is type
punctuation, not an operator.

| Between | The run |
|---|---|
| a binary operator and each operand: arithmetic, comparison, `<=>`, assignment and every compound assignment, `&&`, `\|\|`, `and`, `or`, `xor`, bitwise, `.`, `??`, `\|>`, `is`, `as` | one space |
| `=>` and each side, in an array, a `match` arm and an `fn` | one space |
| a ternary's `?` and `:` and their operands | one space; a ternary with no middle operand writes `?:` with nothing between |
| a unary operator (`!`, unary `-` and `+`, `~`, `++`, `--`, `@`, spread `...`) and its operand | nothing |
| a `,` | nothing before it, one space after it |
| a `;` | nothing before it; inside a `for` header, one space after it unless a `;` or `)` follows |
| an opening `(` or `[` and what follows it; a closing `)` or `]` and what precedes it | nothing |
| a callee and its `(`: a name, a variable, a `)` or `]`, `fn`, a declaration's name, and a keyword the parser reads as a call (`isset`, `empty`, `unset`, `exit`) | nothing |
| a value and its index `[` | nothing |
| `if`, `elseif`, `while`, `for`, `foreach`, `switch`, `match` or `catch` and its `(` | one space |
| the `:` of a return type, a named argument, an anonymous object's field and an enum's backing type | nothing before it, one space after it |
| the `:` that ends a `case` or `default` label | nothing before it |
| `->`, `?->` and `::` and each side | nothing |
| inside a written type: `?`, `\|`, `&`, `<`, `>`, `(` and `)` and their neighbours | nothing; a `,` inside a type's arguments is the `,` row |
| `#[` and what follows it, and what precedes its `]` | nothing |
| a keyword and the word, variable, literal, `(` or `{` after it: `return $x`, `new Cart`, `class Cart`, `public int`, `namespace Shop`, `extends`, `throw`, `echo`, `yield` | one space; nothing before `;` or `::` |
| two words: a type and a variable, a modifier and a type, a name and `extends` | one space |
| code and a comment after it on the same line | one space |

Three runs stay the author's. A run after a block comment that shares its line with code is the
comment's. A run that touches `<?nvs`, `<?=` or `?>` sits beside markup the program prints. And a run
between two tokens whose pair the table does not name is left as it is, so a new token kind is never
spaced by a guess. A test lists every `TokenKind` the lexer has and fails naming any kind that is
neither in the table nor on the short list of kinds whose runs are the author's.

**A run is never emptied where its two tokens would lex as one.** `- -$x` keeps its space, because
`--$x` is a decrement, and so does `> >` closing two type arguments. The invariant behind it is checked
over the whole corpus: a formatted file lexes to exactly the token kinds its source did.

The table is PER-CS 2.0's, with "at least one space" read as exactly one. "At least one" leaves two files
that differ only in spacing still different after formatting, which is what
`rule:tooling/fmt-is-one-canonical-style` exists to prevent.

The cost is stated and accepted: hand alignment is gone. A column of `=`, `=>` or trailing comments
lined up with extra spaces is written with one space each, the first time the file is formatted. What
that buys is that a one-line change never re-aligns the lines around it, so a diff shows only the line
that changed. Formatting stays one pass over the file's tokens, beside the tree the printer already
walks, and nothing of it runs outside `nvs fmt` and the editor's formatting request.

**Declined:**
- **Keeping the author's spacing inside a line.** Two programs that differ only in spacing stay
  different after formatting, and a reviewer still reads `$x=$a+$b` beside `$x = $a + $b`.
- **Alignment of `=`, `=>` and trailing comments, as php-cs-fixer's `align_*` options do.** One edit
  re-spaces every neighbour, and each aligned block is a second layout a reader has to learn.
- **"At least one space", as PER-CS words it.** It is two styles, and the formatter has one.
- **A space after `fn`.** `rule:tooling/fmt-novis-constructs` writes `fn(int $x): int`, and the corpus
  mostly does.
