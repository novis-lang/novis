A `function` declaration and a `const` declaration are legal inside a class body and nowhere else.
Every callable is a `static` or instance method; every constant is a class constant. A top-level
`function foo() { ... }` and a top-level `const FOO = 1;` are each a diagnostic naming the
replacement, and a bare call is a diagnostic that points at the `Core` namespace, where the built-in functions are
methods — `Core\Str::length($s)`.

Novis is OOP-only, and a free-floating name is the same shape of problem
`rule:statements/static-is-a-member-modifier` already closed for state: reachable from everywhere,
declared nowhere in particular, and colliding at load order rather than at a declaration site. With
this closed, the resolver has exactly one kind of entry to answer a call or a constant against, and
no bare-name fallback exists in it at all.

Two things are untouched. An anonymous or arrow function is a value, not a named declaration, and
creating one anywhere is unaffected. A script's own top-level statements are its body, not a
declaration at file scope. What it costs is a class name on every call to a library function, on top of the annotations `rule:types/declaration` already asks for.
