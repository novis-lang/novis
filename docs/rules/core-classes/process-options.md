`ProcessOptions` carries three fields and no more: a working directory, an environment, and a
timeout.

`env`, when given, **replaces** the child's environment entirely rather than merging with the
parent's — explicit replacement is simpler to reason about than merge semantics. Every key and value
is plain `string`, so an API key held as a `secret` needs `rule:core-classes/secret-reveal` first;
this is a new sink reusing an existing escape hatch, not a new mechanism.

`timeout` reuses the existing safepoint-driven cancellation — the same poll that already cancels a
request — rather than a bespoke process-only timer. On expiry the child is killed and the suspended
coroutine resumes into a throw naming the timeout.

**Not shipped.** `crates/nvs-stdlib/src/process.rs` registers `run` and `spawn` with a path and an
argument array and nothing else; there is no options type, so a child inherits the environment, runs
in the calling process's directory, and is bounded only by the request's own wall-clock deadline. It
lands on both members at once when it lands, there being no reason for one of them to take a working
directory the other does not.
