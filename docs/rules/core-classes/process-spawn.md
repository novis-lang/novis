`Core\Process::spawn` takes the same path and argument array as `run` and answers a
`Core\Process\Handle` instead of waiting: `readStdout`, `readStderr`, `writeStdin`, `wait`, `kill`.
Every read and write suspends the calling coroutine exactly as `run`'s wait does, so streaming a
child's output into a response costs one coroutine and no worker thread.

One handle covers both streaming a child's output straight through and full pipe control, because the
difference between them is which members a caller happens to use, not two kinds of process.

A read answers `null` at the end of its stream and a chunk otherwise, never a line and never the
whole output. `wait` closes the child's standard input first, drains what neither read has taken, and
answers the same `Core\Process\Result` a completed `run` does — so a program that streamed everything
gets two empty captures, and one that streamed nothing gets what `run` would have given it. Standard
input is the one place in this class a `tainted` value is accepted: what goes down it is data the
child parses on its own terms, where a path and an argument are a command this process builds.

**A child never outlives the task that spawned it.** One still running when its task ends is killed
and reaped, so memory and processes alike stay O(in-flight) rather than O(children ever started), and
a handle the program simply stops reading from leaves nothing behind.

Both members take the same options bag, `rule:core-classes/process-options`, and its `timeout` bounds
every handle member that parks.
