A job that exceeds its memory, CPU or time budget has had a **failed attempt**. It is recorded as
that, retried on the same ladder as any other failure
(`rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`), and never reported as an
out-of-memory — the job is the unit torn down, and the worker keeps claiming.

A fatal error inside a job follows the ordinary escalation ladder
(`rule:errors/escalation-ladder`) with the job as that unit, so a job whose script does not resolve at
all is a failed attempt too rather than a second policy written beside the first. One shape covers a
throw, a refusal and a budget teardown, which is why a dead-letter row can hold any of the three
without a second entry shape.
