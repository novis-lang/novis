A reserved word is offered only at a cursor where the compiler accepts it, which two things decide. **The
token before the name being written** says whether a statement starts there: after `;`, `{`, `}`, `)`, `:`,
an open or close tag, `else` or `do`, or at the top of the file, every word may be written; after anything
else the cursor is inside an expression and is offered only the words that open one — `new`, `match`, `fn`,
`null`, `true` and the rest of the grammar's primary-expression dispatch — and never `class`, `if` or
`return`. After `new`, `extends` and `implements` it is offered types and no word at all. The tokens are
the lexer's own, so a `;` inside a string is not a statement's end; the tree cannot answer this, because
the name being written is usually what stops the statement around it from parsing.

**What encloses the cursor** decides the rest, read off the index's ancestor list
(`rule:ide/the-index-answers-the-cursor`): `break` needs a loop or a `switch` and `continue` a loop, with
no function body between it and the cursor; `self`, `parent` and `static` need a class, an interface or an
enum; `yield` needs a function body; and the words that open a declaration — `class`, `interface`, `enum`,
`abstract`, `final`, `namespace`, `use`, `autoload` — need there to be no function body around them, since
a type declared inside one is `E0233`.

The lists stay what they were: spellings read out of the grammar's two dispatches, held to the lexer's
reserved words by a test. What this adds is when each is offered. It errs toward offering: after a `:` a
ternary's last operand and a `case` arm's first statement are one position, and both lists are offered
there.
