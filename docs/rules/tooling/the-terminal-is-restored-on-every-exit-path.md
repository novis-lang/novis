Raw mode, a hidden cursor and a live region must be undone on a throw, on a fatal, on an internal
panic and on a signal. Restoration is an obligation of `rule:errors/escalation-ladder`, not a
`finally` an author remembers: a ladder that protects the process while leaving the operator's shell
in raw mode has failed at the thing it exists for. This is the most-forgotten defect in
cross-platform terminal code, and it is the reason in-place output is scoped rather than free-form
(`rule:tooling/in-place-output-is-a-scoped-live-region`) — a region has an end, and the runtime is at
that end on every path.

A statement after the body cannot discharge the obligation, because a throw skips it and a panic
skips every statement there is (`rule:errors/panics-bypass-user-code`). So the end of a region is a
destructor, in the stdlib's scope guard and in the runtime's region both — one ends the *scope*, the
other puts the *terminal* back — and an outer region closing also closes any inner one whose own
guard was skipped, so the stack can never keep a region nobody can reach. What is left on screen is
the last frame; what is restored is the cursor, and the row below the region is where the next `echo`
lands.
