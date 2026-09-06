A denied capability throws a `RuntimeError`, and a program may catch it and degrade. It is not an
escalation: a limit breach is fatal because the request has already consumed something it cannot give
back, whereas a capability denial is known *before* any work is done and leaves nothing behind. A
cache that falls back to recomputing when writing is not granted is a reasonable program, and making
the refusal uncatchable would forbid it (`rule:errors/escalation-ladder`).

No new class is added: `RuntimeError` is "the world said no", where the world is the operator. The
message names the capability **in its configuration spelling**, and for a scoped one the argument that
fell outside the grant — because the reader of that message is usually the operator, and the grant
name is the string they will add to their configuration.

A predicate that would otherwise answer `false` is refused rather than answered, so an ungranted
deployment never looks like a negative result.
