`exit` and `exit(n)` end the program from any frame. No `catch` sees the ending, and no `finally`
between the call and the root runs, in the frame that called it or in any frame above. `exit(n)`
ends with status `n`; `exit("message")` writes the message and ends with status `0`. An operand that
is neither an `int` nor a `string` is a type mismatch at the operand, because
`rule:types/conversion` has no implicit conversion to offer there.

Running no `finally` is PHP's own behaviour for `exit`, checked against PHP's output rather than
assumed, so it is what a program observes and not a simplification. A program that has cleanup to run
on the way out uses `Core\Script::finish()` instead, and one that must act on every ending registers
an exit hook (`rule:observability/three-endings-fire-the-exit-queue`).

`exit` is a status of its own on `rule:errors/propagation`'s calling convention, `EXITED`, beside
`OK`, `THROWN` and `FATAL`, and the status `n` it named travels on the request context rather than in
the status word. It shares propagation with `FATAL`: neither is catchable, because a `catch` admits
only `THROWN`, and neither runs a `finally`, because every `finally` sits behind that same
comparison. It does not share what a `FATAL` means. A `FATAL` is a failure, escalated and reported at
the request boundary as one (`rule:errors/escalation-ladder`), while `exit(0)` is the most ordinary
way a program ends, so the boundary reports the status and not an error. The frame's locals are
still released, because `exit` is an ordinary helper call carrying the error edge every call has.

- **Memory:** one 64-bit integer per request, the status `exit` named. Every status check already
  compares against `OK`, so the extra status adds no instruction to a call site.

**Declined:**
- **`exit` as a `FATAL` carrying a code.** Every clean exit would be reported as an internal error,
  and the escalation ladder would run for a program that ended normally.
- **`exit` as a `throw` of a dedicated class.** A `catch (Throwable)` or a `finally` would then
  observe it, which PHP's `exit` never allows.
