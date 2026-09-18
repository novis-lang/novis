`Core\Ast::parse` calls directly into the same lexer and parser the compiler itself runs, so a
construct that parses when a file is compiled parses identically when a running program parses the
same text, and a rejected construct is rejected identically in both places. There is no second
grammar implementation anywhere in the project.

**Parsing a file is that one member composed with `Core\IO::read`, and there is no `parseFile`.** A
path is the filesystem's question, so asking it through a second member would put a `fs.read` check
somewhere other than the door that already owns one (`rule:security/capability-check-at-the-door`),
and leave the class holding a capability for the sake of one spelling. Two calls keep `Core\Ast`
capability-free, which is what `rule:security/reflection-needs-no-capability` rests on.

The return value is a **typed** node tree — one class per production, named for the production the
grammar's own walk names — never an untyped array or a stringly-keyed structure. Handing back the
parse tree as untyped data would be exactly the shortcut `token_get_all()` takes, reintroduced at the
one place a fully-typed alternative is easiest to give. Those classes are identity rather than
surface: none carries a registry row, so no source can name one in a type position, and a node's
production is what `Core\Reflect::forObject` answers and `kind()` is the same production spelled
short.

**A node says where it is and never what it says.** `line()`, `column()` and `offset()` name the
first character of the production — the line and column counted from 1, the column in characters and
the offset in bytes — so a `#[Test]` that walks the tree reports a `file:line` rather than only a
verdict, and a structural rule becomes a report rather than a check. No member answers a node's own
source text: the position is derived from the argument rather than carried out of it, which is what
keeps `parse`'s `$source` qualifier-neutral, and a caller wanting the text holds the string it passed
and slices it at the offset itself.

**A parsed tree is inert. There is no path from an AST value back into execution.** `eval` does not
exist and stays rejected: a string has no stable identity, no cache key, and no capability-grantable
path. A program can walk a tree, print it, or rewrite it into a new source string to hand to a human
or a file — never a way to run what it describes. Shipping this is therefore not `eval` under a
different name; it is the same refusal restated.
