A `class`, `interface` or `enum` is declared at file scope, or not at all. A declaration written
inside a method body, a property hook, a closure body or a nested block at file scope is `E0233` where
it is written. An anonymous class (`new class { … }`) is the same declaration in expression position,
and is `E0244`.

Every type name resolves against a static table built before any code runs: the `require` and
autoload graph, then the member and hierarchy tables, then the field layouts. None of those has a
reading for a name that may or may not exist yet. `nvs_types::locals`' per-body walk reports `E0233`,
and it is reached only from inside a body, so it needs no test of whether it is nested.

What it costs is the conditional-class idiom. Its two real uses, a polyfill and a feature switch, are a
`require` of one file or the other, which is static and already works. An anonymous class has no name
for the static table to hold, and its two real uses, a one-off implementation of an interface and a
test double, are a named class in the same file or a closure.

**Declined:**
- **Declaring a nested type unconditionally at file scope.** It silently changes what the program
  says: `if ($legacy) { class Session { … } }` would declare `Session` whether or not the branch runs.
- **A conditional entry in the class table.** Every name resolution, layout and dispatch decision
  below it would carry a run-time question, paid at every lookup instead of once here.
