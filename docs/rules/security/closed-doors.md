Four mechanisms that dynamic web runtimes commonly have are absent from Novis by construction: native code loading
(`rule:security/no-ffi`), scheme dispatch on a path (`rule:security/a-path-is-not-a-url`),
cross-request state (`rule:security/no-cross-request-state`) and `eval`
(`rule:security/no-eval`). Each closes a vulnerability class outright rather than defending against
it.

None of the four has a correct tier: each is unsafe in-process, unavailable in a sandbox, and unsafe
as a build-time option, so asking which tier it belongs at is a category error. Each also has a real
constituency in PHP, and none is closed for being unused; each trades away something ranked above
it. They are written down as rules because a mechanism closed only by not having been built yet is
an absence, and an absence gets filled by whoever needs it next.

**Nothing here is revisitable by configuration.** There is deliberately no directive for any of the
four, because an option that is off by default is still a mechanism that exists, and every safety
claim downstream would have to be qualified with "unless it is enabled." That is the difference
between a door that is shut and a door that is locked.

The cost is named rather than minimised. A class of library cannot be ported at all — anything whose
whole purpose is process-global state or in-process native binding — and cross-request coordination at
shared-memory latency is not available at any price. That spends latency to buy isolation, which is
the ordering `rule:programs/memory-priority` mandates.
