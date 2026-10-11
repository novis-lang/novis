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
zero word rather than a shorter argument list. The slot is filed under the call and not the literal
because the lowering holds the call's span, and keying on the literal would make it map the written
arguments back to parameters, which only the checker holds the names for
(`rule:core-api/parameters-are-callable-by-name`). The slot is always written because a member's
helper reads one fixed argument list: a constant emitted only where something was prepared would have
it read its first argument from the caller's next slot.

A word the running build does not know is read as "nothing was prepared"
(`crates/nvs-stdlib/src/regex.rs:1288`). Refusing it would turn a compiler-version skew into a failed
request, and the artifact's address already names the compiler build, so a cached unit cannot hand a
process a word another compiler minted.

A **live object is never what crosses**. A compiled pattern belongs to the core that built it, so what
travels for a pattern given as a string literal is the engine tier it settled in and the runtime spends the compile. That
bound is what keeps this rule's first paragraph true across the boundary: the prepared path hands the
runtime path an earlier answer to a question it would have asked itself, never a different
implementation of it.

One observable difference is named rather than denied: a fully folded call is no longer a call, so the
per-call probe does not fire for it. A *prepared* call — the common case — is still a call and probes
normally, and the enclosing statement's probe is unaffected either way.

- **Memory:** one constant per call site on a roster member, and one machine word in the argument list
  of the members that take it. No allocation, no data section and nothing per request; a member off the
  roster is unchanged instruction for instruction.

**Declined:**
- **A table of prepared answers on the compiled program, interned and pointed at.** It is the shape for
  a prepared plan, and the wrong one for a fact that fits in the slot: it buys an indirection and an
  entry per distinct pattern to carry two bits. A fact that needs a table can add one beside this slot.
- **Seeding the per-core pattern cache when a unit loads.** It needs no new argument, but a unit loads
  once and cores are many, so the seeding is either cross-thread or repeated per core, and it only
  pre-fills a cache keyed by text, which not every grammar has.
- **Folding eagerly, evaluating whole calls wherever possible.** The calls worth touching mostly
  depend on runtime state, so an eager rule would either be wrong or need this same carve-out, stated
  less clearly.
