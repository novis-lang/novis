A `class`, `interface` or `enum` is declared at file scope, or not at all, with one exception: an
`enum` may also be declared in a class or interface body (`rule:enums/class-scoped-enum`). A
declaration written inside a method body, a property hook, a closure body or a nested block at file
scope is `E0233` where it is written, and its help names file scope and, for an enum, the class body.
A class or an interface in a class body is one `E0233` too. An anonymous class (`new class { … }`) is
the same declaration in expression position, and is `E0244`.

Every type name resolves against a static table built before any code runs: the `require` and
autoload graph, then the member and hierarchy tables, then the field layouts. None of those has a
reading for a name that may or may not exist yet. `nvs_types::locals`' per-body walk reports `E0233`,
and it is reached only from inside a body, so it needs no test of whether it is nested. A class body
is not a body that runs: an enum declared there exists exactly when its owner does, so it goes into
the same static tables, keyed by its owner. A nested class would need a layout and a dispatch table
inside another class's, and nothing reads one there.

What it costs is the conditional-class idiom. Its two real uses, a polyfill and a feature switch, are a
`require` of one file or the other, which is static and already works. An anonymous class has no name
for the static table to hold, and its two real uses, a one-off implementation of an interface and a
test double, are a named class in the same file or a closure.

**Declined:**
- **Declaring a nested type unconditionally at file scope.** It silently changes what the program
  says: `if ($legacy) { class Session { … } }` would declare `Session` whether or not the branch runs.
- **A conditional entry in the class table.** Every name resolution, layout and dispatch decision
  below it would carry a run-time question, paid at every lookup instead of once here.
- **A class or interface in a class body, as the enum is.** It would bring members, a layout and
  inheritance inside another class's, for a grouping a namespace already gives.
