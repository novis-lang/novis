`Core\Ast::parse` and `::parseFile` call directly into the same lexer and parser the compiler itself
runs, so a construct that parses when a file is compiled parses identically when a running program
parses the same text, and a rejected construct is rejected identically in both places. There is no
second grammar implementation anywhere in the project.

The return value is a **typed** node tree — one type per production — never an untyped array or a
stringly-keyed structure. Handing back the parse tree as untyped data would be exactly the shortcut
`token_get_all()` takes, reintroduced at the one place a fully-typed alternative is easiest to give.

**A parsed tree is inert. There is no path from an AST value back into execution.** `eval` does not
exist and stays rejected: a string has no stable identity, no cache key, and no capability-grantable
path. A program can walk a tree, print it, or rewrite it into a new source string to hand to a human
or a file — never a way to run what it describes. Shipping this is therefore not `eval` under a
different name; it is the same refusal restated.
