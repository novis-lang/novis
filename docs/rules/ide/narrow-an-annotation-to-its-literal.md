A deep nested literal gets annotated `array<mixed>` because writing `array<array<array<float>>>` by hand
is tedious, and the annotation then costs the program the typed path for the rest of its life: every
operation on a `mixed` resolves through the generic helper path, every element write becomes a runtime
check, and every read out of the structure needs an `as` at the far end. The type is derivable — the
checker already walks every element — so one editor action derives it.

The action is offered on the **type annotation** of a declaration whose initializer is an **array literal
in the same file**. It rewrites that annotation and nothing else — not the literal, not a use site, not
another line. It is invoked by the developer from the annotation; there is no diagnostic and no hint
behind it, because nothing is wrong, and discoverability rides on the annotation's own light bulb. It
never changes a value's kind: a union in the offer such as `array<string|int|array<string>>` is a signal
that the value is a record and wants an object shape, but converting the literal is not this action's
business, because it would turn copy-on-write value semantics into shared-reference ones.

What it writes is decided by `rule:ide/narrowest-means-narrowest-base-type`; when it declines, by
`rule:ide/the-action-answers-from-the-literal-or-not-at-all`; and why it never runs on save, by
`rule:ide/narrowing-is-a-diff-never-a-save-time-fix`. It is a code action whose fix the checker has to
compute, which places it under `rule:ide/a-code-action-writes-only-what-is-already-determined` and on
the far side of the boundary the first editor slice draws.

A round trip asserts it: offered on a literal-initialized annotation, absent on a `Core\Json::decode`
initializer, and the file it produces still checks clean.
