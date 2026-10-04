Every binding site carries a type, and only a local and a `foreach` binding may take theirs with `var`
rather than writing it. PHP's existing slots become mandatory — parameter, return (`void` and `never`
included), property, promoted constructor parameter, class constant, anonymous function parameter and
anonymous function return, `catch`, enum backing type — and four positions PHP has no slot for get one: a local at its
declaration, a `foreach` key and value, a `for` header's init clause
(`rule:iteration/for-init-clause`), and a destructuring target. A local, a `for` init declaration and
a `foreach` key or value may write `var` instead, which takes the type from the expression that fills
the binding (`rule:types/var-inference`); nothing else may omit a type.

The return slot is owed by every declaration a caller reads, an abstract method and an interface
member included, and **the constructor is the one exception**: it answers with the instance rather
than with a value, which is why a valued `return` in one is refused, so it writes no return type and
a written `: void` there is accepted while saying nothing the declaration did not. An
expression-bodied anonymous function is the other place the slot may stand empty, and for the opposite reason —
its body is a single expression, which is its own answer, while a block-bodied one owes the
annotation like any method (`rule:types/anonymous-function`).

A binding is declared **once**. A later assignment is bare, and is legal only where the name is
already declared in the enclosing function; re-declaring a live name is a diagnostic naming the first
declaration, and there is no shadowing. Declaration is function-scoped as in PHP — a binding declared
inside an `if` is visible after it — but *definite assignment is checked*: reading a binding on a path
that may not have reached its initialiser is a compile error, not PHP's warning and a `null`.

A declared type is then fixed for the binding's whole life, whether it was written or taken by `var`.
No assignment, operator or call changes it; `settype()` joins the rejected list with a diagnostic
naming `as` (`rule:types/conversion`). An `inout` binding ties two names to one slot, so both sides
declare the **same** type (`rule:statements/inout-is-the-by-reference-spelling`); an alias that widens
or narrows is a diagnostic. There is no function-scope `static` and no global constant, so neither has
a binding site at all (`rule:statements/no-function-static-and-no-global`).
