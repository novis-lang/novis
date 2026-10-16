A pattern argument given as a string literal is validated and compiled while checking, under
`rule:expressions/intrinsic-constant-arguments`, and the tier it landed in is written down where later stages
read it back. The tier is settleable there because it is a property of the pattern text alone —
decided by which engine's parser refused a construct — so a checking run and a request cannot
disagree about it.

Three consequences follow, none of which costs anything at run time. **A malformed pattern is a
compile error**, not a run-time throw on the first request that reaches it. **The tier is known
statically**, so a check run can report which patterns require backtracking. And **an operator can
refuse them**: `[regex] backtracking = "allow" | "warn" | "deny"` makes a backtracking pattern
respectively silent, a warning, or a compile-time error, so a deployment running untrusted or
high-volume code can know no request can be made to backtrack at all.

A pattern assembled at run time is compiled at run time and gets the same tiering and the same
budget, with none of the three benefits. That is a reason to write patterns as string literals, stated here
rather than discovered.

The third consequence is not built: there is no `[regex]` block, so `deny` cannot be written yet.
