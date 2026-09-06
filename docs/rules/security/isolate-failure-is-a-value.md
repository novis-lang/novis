A child's failure arrives as **data on the result**, never as an exception unwinding into the parent.
A top-level `return` gives `ok = true` and a copy of the value; an uncaught throw gives `ok = false`
and an `error` carrying the class name, message, code and rendered trace **as copied data, not the
exception object**; a limit breach names the directive it broke; a contained runtime panic leaves the
parent running, which is a strict improvement over losing a process. A cancelled parent cancels the
child at its next safepoint and drops its arena, so there is no orphan.

Rethrowing the child's exception object was rejected: it would have to copy an arbitrary object graph
across a boundary that forbids exactly that (`rule:security/isolate-values-cross-by-copy`), and it
would encourage reading a spawn as a function call. Code that wants the terse form asks for it, and
the diagnostic then names the isolate rather than a parent frame it never had. Nothing unwinds across
the boundary, for the reason nothing unwinds across a JIT frame (`rule:errors/propagation`).
