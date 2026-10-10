Arguments in and the result out are **deep-copied, or moved when the refcount is 1** — the identical
mechanism and identical restrictions as `spawn worker`. It is a *graph* copy, not a tree copy: shared
substructure stays shared and cycles terminate, so `$a['self'] = $a` crosses instead of hanging.

Three things cannot cross. A **callable** captures a heap and a scope; an **`inout` binding** is an
alias; an object holding a **host handle** — an open file, a socket, a child process — is owned by
this process. None of the three has a meaning in another heap. The idiom is to pass what *identifies*
a resource, not the resource: a DSN, a path, a credential reference, and the child opens its own. An
object whose class the receiving side cannot resolve is refused with a diagnostic naming the class,
never degraded into a stub. A `secret`-typed property is refused there too
(`rule:security/secret-crosses-no-boundary`).

A value crosses exactly twice, and both crossings are the same walk. In, at the spawn, the `args:`
value crosses from the parent's ownership root into the child's. Out, at the `await`, the child's
top-level return value, its captured output and, on failure, its error as data cross back. Nothing
else moves: the parent's output buffer, the backing state of its request accessor classes
(`rule:statements/no-host-populated-variables`) and any open handle are not reachable from the child's
root, and neither crossing makes them reachable.

The out crossing runs before the child's root is released. Both contexts allocate from the same place
(`rule:security/arena-is-an-ownership-root`), so a move at refcount 1 is a pointer handoff with no copy
and no bookkeeping, and a child returning a large array costs the walk and not the array.

There is one walk and one set of refusals, which is why it is the audited surface. Two boundaries
with different value rules would be two sets of rules for a developer to learn and two implementations
to keep correct, and a refusal at the boundary is a diagnostic where the mistake was made rather than a
surprise later. Across cores the
walk is the same but the **move is not available**: refcounts are non-atomic because a value is
reachable from one core only, so a cross-core crossing copies at every node. That cost belongs to the
placement option, not to the boundary.
