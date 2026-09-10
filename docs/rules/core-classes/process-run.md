`Core\Process::run(string $path, array<string> $argv, ProcessOptions $options)` spawns the process,
waits for it to exit, and answers a result carrying the exit code and both captures. Waiting is an
ordinary suspension point on the runtime's stackful coroutines — the same mechanism that lets any
function perform I/O without being marked async — so a slow child ties up one coroutine's stack and
not the worker thread it started on, and other requests on the same core keep making progress.

**Captured stdout and stderr are `bytes`, never `string`.** An arbitrary child's output cannot be
assumed valid UTF-8, so a caller who knows it is text writes `as string`, which throws on invalid
input rather than mangling it into replacement characters. The two captures stay apart, a non-zero
exit keeps both, and a result answers the same thing every time it is asked.

What it spends, per call: the child's whole stdout and stderr, once each, held for as long as the
program holds the result, plus one object allocation — charged to the request that asked. That
capture is bounded by the request's existing `max_output` directive rather than by a cap of this
member's own: the same number the response ceiling is, read once per call, and a child that keeps
writing past it is killed while `run` throws.

**That refusal is catchable, and is not `rule:errors/on-limit`'s `FATAL`.** Nothing reached the
response, so the request exceeded no limit — the member declined to hold more than the request is
allowed to produce, which is a refusal in `rule:security/denial-is-a-runtime-error`'s shape. A caller
who ran a chattier child than it meant to can catch it and run a different one. `Core\IO::read` holds
a file to the same directive out of the same pair of methods, so the two answer a program alike.
