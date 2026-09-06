The definite-assignment pass runs inside every method, not only constructors, with a `lateinit`
property entering in the "not yet proven written" state. A read reachable from the method's entry with
no intervening write to that property *and no intervening call at all* is a compile error.

A call is opaque and immediately moves the property to "assumed written". The analysis cannot know
whether the callee wrote it, and staying silent is the sound direction: a check that cries wolf on
legitimate injected code would cost more trust than it returns. So the diagnostic fires only on the
narrowest shape — a straight-line or branchy, call-free stretch that reads before it sets — and says
nothing about the cross-method, cross-object case `lateinit` exists for, which relies entirely on the
runtime throw.

Nothing interprocedural is attempted. Whole-program analysis would widen the blast radius of every
edit under per-file hot reload, for cases that fall through to the runtime throw anyway. This check is
a bonus, not a safety net to rely on.
