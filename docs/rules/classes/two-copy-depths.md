Novis has two copy depths and no third. `clone` is the shallow, same-heap, one-level copy
(`rule:classes/clone-is-shallow`). The graph copy is recursive, cycle-safe and heap-crossing
(`rule:classes/graph-copy`), and it is one operation reached by two carriers — the isolate boundary
and `Core\Serialize`. Neither depth is customizable by a class: there is no `__clone`, no
`__serialize`, no `__unserialize`, no `__sleep` and no `__wakeup`.

Keeping both is deliberate, because it is a real distinction: cloning a tree node should not
deep-copy what it references, and forcing `clone` deep would silently change every class that relies
on shallow-copy-then-shared-reference.

A copy therefore always means what the language says it means, which is what closes the
deserialization gadget-chain class by construction — no hook fires during reconstruction, so there is no
method call for attacker-controlled property values to drive. The cost is a real capability: a class
that wants a duplicated nested collection or custom versioning has to expose an explicit method and
call it, and two copy depths remain two things to learn.
