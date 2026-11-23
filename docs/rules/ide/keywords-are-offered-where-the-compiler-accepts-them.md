A reserved word is offered only at a cursor where the compiler accepts it, which two things decide. **The
token before the name being written** says whether a statement starts there: after `;`, `{`, `}`, `)`, `:`,
an open or close tag, `else` or `do`, or at the top of the file, every word may be written; after anything
else the cursor is inside an expression and is offered only the words that open one — `new`, `match`, `fn`,
`null`, `true` and the rest of the grammar's primary-expression dispatch — and never `class`, `if` or
`return`. Where a type is written it is offered types and no word at all, and after `new` the classes
`new` compiles on beside `self`, `static` and `parent` inside a class
(`rule:ide/an-accepted-type-writes-what-follows-it` is which tokens say so). The tokens are the lexer's own, so a `;` inside a string is not a statement's end; the tree cannot answer this, because
the name being written is usually what stops the statement around it from parsing.

**What encloses the cursor** decides the rest, read off the index's ancestor list
(`rule:ide/the-index-answers-the-cursor`): `break` needs a loop or a `switch` and `continue` a loop, with
no function body between it and the cursor; `self`, `parent` and `static` need a class, an interface or an
enum; `yield` needs a function body; and the words that open a declaration — `class`, `interface`, `enum`,
`abstract`, `final`, `namespace`, `use`, `autoload` — need there to be no function body around them, since
a type declared inside one is `E0233`.

**`attribute` and `repeatable` open an attribute declaration**
(`rule:attributes/an-attribute-is-declared-with-its-sites`). Both are contextual words, so they are a
list of their own beside the reserved ones, and a test holds them out of the lexer's. They are offered
where a statement starts with no function body around it, and in a class or an interface body where a
member starts with no modifier before it. An enum body never offers them, because a declaration there
is `E0142`. After `repeatable`, `attribute` is the one word offered. With a client that takes snippets,
`attribute` writes `attribute ${1:Name} on ${2|class,method,property,parameter|} { $0 }`, and
`repeatable` writes the same after `repeatable `; the site choice is the four words a program may list.
Without snippets, each writes its word. Inside the declaration's `{ … }`, a field's type is offered the
types a property's type is, and a field's name, after the `{` or a field's `,`, is offered nothing.

The lists stay what they were: spellings read out of the grammar's two dispatches, held to the lexer's
reserved words by a test. What this adds is when each is offered. It errs toward offering: after a `:` a
ternary's last operand and a `case` arm's first statement are one position, and both lists are offered
there.
