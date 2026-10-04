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

There is one walk and one set of refusals, which is why it is the audited surface. Across cores the
walk is the same but the **move is not available**: refcounts are non-atomic because a value is
reachable from one core only, so a cross-core crossing copies at every node. That cost belongs to the
placement option, not to the boundary.
