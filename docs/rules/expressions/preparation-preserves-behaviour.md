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

**What preparation stores is a fact about the text, and it reaches the member as an argument.** The
checker files what it prepared under the call's own span, the lowering reads it there and emits it as
a leading constant of that call, and the word rides in the unit's code — so the artifact carries it
with nothing serialized beside it, and a member is handed the answer instead of deriving it again.
The roster of members taking one is closed and lives beside the intrinsic roster, so the slot is a
cost only the members that read it pay; a member on it whose argument was not a literal is handed the
zero word rather than a shorter argument list.

A **live object is never what crosses**. A compiled pattern belongs to the core that built it, so what
travels for a pattern given as a string literal is the engine tier it settled in and the runtime spends the compile. That
bound is what keeps this rule's first paragraph true across the boundary: the prepared path hands the
runtime path an earlier answer to a question it would have asked itself, never a different
implementation of it.

One observable difference is named rather than denied: a fully folded call is no longer a call, so the
per-call probe does not fire for it. A *prepared* call — the common case — is still a call and probes
normally, and the enclosing statement's probe is unaffected either way.
