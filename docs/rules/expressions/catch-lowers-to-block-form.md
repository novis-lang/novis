There is no run-time representation of the expression form. It lowers to the block form's lowering —
push the protected region, lower a body, take the thrown reference in the handler block, dispatch by
a chain of class tests, re-raise what no arm matched, join — differing in exactly one way: the body
and each arm produce a value, which is written to one temporary and joined by a phi, the way `match`
joins its arms.

Nothing in the runtime or in codegen changes, and the landing pad is the one already emitted
(`rule:errors/propagation`). The cost of a guarded expression that does not throw is the block form's:
zero on the happy path (`rule:errors/throw-is-not-slower`).
