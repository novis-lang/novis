Every binding site carries a type, and only a local and a `foreach` binding may take theirs with `var`
rather than writing it. A type is owed at parameter, return (`void` and `never` included), property,
promoted constructor parameter, class constant, anonymous function parameter and anonymous function
return, `catch` and enum backing type, and at four more positions: a local at its declaration, a `foreach` key and value, a `for` header's init clause
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
declaration, and there is no shadowing. Declaration is function-scoped — a binding declared inside an
`if` is visible after it — but *definite assignment is checked*: reading a binding on a path that may
not have reached its initialiser is a compile error.

A declared type is then fixed for the binding's whole life, whether it was written or taken by `var`.
No assignment, operator or call changes it; `settype()` joins the rejected list with a diagnostic
naming `as` (`rule:types/conversion`). An `inout` binding ties two names to one slot, so both sides
declare the **same** type (`rule:statements/inout-is-the-by-reference-spelling`); an alias that widens
or narrows is a diagnostic. There is no function-scope `static` and no global constant, so neither has
a binding site at all (`rule:statements/no-function-static-and-no-global`), and definite assignment
holds with no exception: no binding is live on a call whose path never ran its initialiser.

A declared type is what makes the other priorities cheap. Untrusted input cannot coerce silently into
a wrong but valid value — `"abc"` read as row id `0` — because no binding changes type without an
`as`. The backend emits a native instruction wherever both operand types are known, which is every
declared binding, instead of dispatching through a tag-testing helper
(`benches/abi-probe/tests/perf_guards.rs` holds a typed arithmetic loop to that). And there is one type
system rather than two: no inference engine to keep in agreement with the declarations, no unknown
type in the IR, and no boundary between checked and unchecked code except `mixed`, which is written.
A mistake arrives as a diagnostic at its cause rather than as a wrong value far from it.

The cost is that PHP source does not run unconverted. PHP has no syntax for the type of a local, a
`foreach` binding or a destructuring target, so no PHP file satisfies this rule as it stands.

**Declined:**
- **Gradual typing, declared types checked and locals inferred.** It is two type systems to keep in
  agreement, and an unknown type the IR would have to carry. `var` takes a local's type from its
  initializer once, at the declaration, and the type is then fixed like any other.
- **An omitted return type defaulting to `mixed`.** It would be a second unchecked position, reached by
  writing nothing rather than by writing `mixed`.
