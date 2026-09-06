The never-written marker is not a value a program can hold, observe or name. It is **not in the type
system** — a nullable field's declared type does not contain it, no expression evaluates to one, and the
call site that "writes" it writes nothing at all while the compiler materializes the fill. It is **never
handed to user code**: its only reader is the native helper behind the member, which turns it into the
member's own behaviour before anything returns, and a helper that fails to handle it is a contract
violation in this repository's own code rather than a wrong answer given to a program. And it is
**transient**: it exists for the duration of the call that materialized it, is consumed by the helper, and
is never stored, returned or reachable from a value a program holds.

A **user-declared** function's optional parameter is unchanged, deliberately. `?int $x = null` in program
source still cannot tell an omitted argument from a written `null`, because closing that would need a
spelling for asking the question — an `isset` on a parameter, or a third state a program can observe — and
that is a language-surface decision this one does not open. A native helper can be held to reading three
states by a test; a user's function body cannot be, and giving it a state it has no way to name would be
exactly the observable marker this rule refuses.
