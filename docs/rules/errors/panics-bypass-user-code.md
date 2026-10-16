A resource-limit `FATAL` means the *script* asked for too much. The runtime's own state is fine, so
a request-level handler reacting to it — logging with context, deciding whether the job should be
retried — is meaningful.

An internal-runtime-panic `FATAL` means Novis's own Rust code broke its own invariant. The runtime's
state is then exactly what cannot be trusted, and running more script code on top of it is the
riskier move, not the safer one.

So **only a resource-limit `FATAL` reaches `rule:errors/on-limit`**. An internal panic goes straight
to `rule:errors/handler-script` — still user-formattable, still routed wherever the operator sends
diagnostics — without ever calling back into the failing request's own code.
