`Core\Process::spawn` takes the same path, argument array and options as `run` and answers a handle
instead of waiting: read stdout, read stderr, write stdin, wait, kill. Every read and write suspends
the calling coroutine exactly as `run`'s wait does, so streaming a child's output into a response
costs one coroutine and no worker thread.

One handle covers what PHP splits between `passthru` (stream straight through) and `proc_open` (full
pipe control), because the difference between them is which members a caller happens to use, not two
kinds of process.

**Not shipped.** `crates/nvs-stdlib/src/process.rs` registers `run` alone; there is no handle type,
so a program that needs to interleave with a child's output has no member to reach for.
