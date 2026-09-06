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
program holds the result, plus one object allocation — charged to the request that asked. The record
reuses the request's existing `max_output` directive to bound that capture; nothing reads it in the
tree today, so what bounds a capture is the request's memory limit, which the buffers are charged
against like any other allocation (`crates/nvs-stdlib/src/process.rs`, gap 1).
