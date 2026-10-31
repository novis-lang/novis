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
budget. A malformed one throws at the call that compiles it, and its tier is not known while
checking. That is a reason to write patterns as string literals, stated here rather than
discovered. The operator's refusal holds for it too: under `deny`, a run-time pattern that only the
backtracking engine accepts throws the `RuntimeError` a malformed pattern throws, at the call that
compiles it and before that engine is built, so under `deny` no request can backtrack at all. Under
`warn` and `allow` it runs, and nothing is logged on the request path.

`[regex] backtracking` is a top-level block of one key, `allow` in every mode with nothing written,
because that is how every program ran before it and the step budget already bounds a backtracking
match. It is `System` and `Reload`: it decides what compiles, so no request may set it, and it is an
input of the environment digest (`rule:config/the-extension-set-is-in-every-unit-key`), so a reload
that changes it recompiles every unit. `[[app]]` takes no `[regex]` block. Under `warn` a
backtracking literal is `W1029`, and under `deny` it is `E0893`.
