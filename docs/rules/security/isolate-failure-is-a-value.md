A child's failure arrives as **data on the result**, never as an exception unwinding into the parent.
A top-level `return` gives `ok = true` and a copy of the value; an uncaught throw gives `ok = false`
and an `error` carrying the class name and message **as copied data, not the exception object**; a
limit breach gives `ok = false` with a message naming the limit it broke; a contained runtime panic leaves the
parent running, which is a strict improvement over losing a process. A cancelled parent cancels the
child at its next safepoint and drops its arena, so there is no orphan.

A child that runs to its end without a top-level `return` gives `null`, the value every Novis function
without a `return` produces. It is not `require`'s `1` (`rule:statements/a-require-expression-is-mixed`):
that value belongs to the construct PHP has, and `spawn script` is not one.

Nothing unwinds across the boundary, for the reason nothing unwinds across a JIT frame
(`rule:errors/propagation`): the status is checked, and tearing down the child's arena is explicit code
on the failure path rather than an implicit landing pad.

**Declined:**
- **Rethrowing the child's exception object in the parent.** It would have to copy an arbitrary object
  graph across a boundary that forbids exactly that (`rule:security/isolate-values-cross-by-copy`), and
  it would encourage reading a spawn as a function call. A caller who wants the failure to propagate
  checks `ok` and throws.
