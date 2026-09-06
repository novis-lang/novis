The prepared path and the runtime path **share one implementation**. The compiler calls the same
pattern compiler, the same URI parser and the same format-plan builder the runtime would have called,
and stores the output. A divergence between the two is a bug of the same class as an incorrect
optimisation, never a permitted difference.

**Preparation is not evaluation.** A call whose result depends on runtime state is not evaluated at
compile time; only the state-independent part is. `$now->format("Y-m-d")` prepares the format plan and
formats at run time, because the instant, the timezone and the calendar are all runtime values. The
narrower case where the whole call is constant folds to its value, but that is ordinary constant
folding.

Because preparation depends only on the literal and on the compiler's own version, it is keyed by the
artifact cache like any other compiled output, including on the compiler-environment component — so a
prepared artifact from a different compiler build is a cache miss rather than a mismatch.

One observable difference is named rather than denied: a fully folded call is no longer a call, so the
per-call probe does not fire for it. A *prepared* call — the common case — is still a call and probes
normally, and the enclosing statement's probe is unaffected either way.
