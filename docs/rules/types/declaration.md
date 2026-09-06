Every binding site carries a written type. PHP's existing slots become mandatory — parameter, return
(`void` and `never` included), property, promoted constructor parameter, class constant, closure
parameter and closure return, `catch`, enum backing type — and four positions PHP has no slot for get
one: a local at its declaration, a `foreach` key and value, a `for` header's init clause
(`rule:iteration/for-init-clause`), and a destructuring target. A local may write `var` instead
(`rule:types/var-inference`); nothing else may omit a type.

A binding is declared **once**. A later assignment is bare, and is legal only where the name is
already declared in the enclosing function; re-declaring a live name is a diagnostic naming the first
declaration, and there is no shadowing. Declaration is function-scoped as in PHP — a binding declared
inside an `if` is visible after it — but *definite assignment is checked*: reading a binding on a path
that may not have reached its initialiser is a compile error, not PHP's warning and a `null`.

A declared type is then fixed for the binding's whole life. No assignment, operator or call changes
it; `settype()` joins the rejected list with a diagnostic naming `as`
(`rule:types/conversion`). An `inout` binding ties two names to one slot, so both sides declare the
**same** type (`rule:statements/inout-is-the-by-reference-spelling`); an alias that widens or narrows
is a diagnostic. There is no function-scope `static` and no global constant, so neither has a binding
site at all (`rule:statements/no-function-static-and-no-global`).
