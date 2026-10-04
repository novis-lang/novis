A `secret` value is refused at the one recursive graph-copy operation, for both of its callers alike —
serialization to bytes and every crossing into a task, a worker or an isolate — rather than drawing a
new distinction between "crossing to a live isolate" and "externalizing to bytes."

The refusal has two halves because the walk cannot see everything. At run time the walk refuses a
`secret`-typed property alongside the callables, aliases and host handles it already refuses
(`rule:security/isolate-values-cross-by-copy`). The half a run-time walk cannot see is refused earlier,
by reading a call's **written arguments** while checking; the three spawn forms hand that check their
own argument list rather than growing a second rule.

Where a worker genuinely needs a credential to do its job, the conspicuous reveal before the call is
the intended escape — explicit, greppable, and at the one call site where the decision belongs. That
this may add friction to a worker that exists specifically to isolate credential handling is a stated
cost of keeping one copy operation rather than two.
