The definite-assignment pass runs inside every method, not only constructors, with a `lateinit`
property entering in the "not yet proven written" state. A read reachable from the method's entry with
no intervening write to that property *and no intervening call at all* is a compile error.

A call is opaque and immediately moves the property to "assumed written". The analysis cannot know
whether the callee wrote it, and staying silent is the sound direction: a check that cries wolf on
legitimate injected code would cost more trust than it returns. So the diagnostic fires only on the
narrowest shape — a straight-line or branchy, call-free stretch that reads before it sets — and says
nothing about the cross-method, cross-object case `lateinit` exists for, which relies entirely on the
runtime throw.

Within that shape the error is as certain as any other definite-assignment error, and it reuses the
pass the constructor check already runs, so it adds no second analysis. Most real `lateinit` use never
triggers it: it is a bonus, not a safety net to rely on.

**Declined:**
- **Interprocedural or whole-program checking**, tracing which methods can write a `lateinit` property
  before which read it. The cases it would add are cross-method by nature, and the runtime throw
  already catches every one of them, so a second analysis would buy no safety.
