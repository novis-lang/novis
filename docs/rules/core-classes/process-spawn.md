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

**A child ends when the program that spawned it ends** — the script under `nvs run`, the request
under `nvs serve` — on every platform, and also when nvs itself is killed. One still running then is
killed and reaped, so memory and processes alike stay O(in-flight) rather than O(children ever
started). **That is an error, not a silent stop, when it cut something off:** the child was still
running and the program never called `wait()` or `kill()` on its handle. A child that already exited
on its own is not one, and `run` always waits, so it never is. The program has ended, so the error
cannot be caught; it is reported the way an uncaught throw is — an `error` record on standard error
and exit status `1` under `nvs run`, an `error` record in the request's log under `nvs serve`, whose
response is not rewritten — and after the throw or `FATAL` that ended the program, when one did. A
`spawn` call written as a statement of its own throws its handle away, so it is the one form of this
mistake the compiler can see, and it does not compile (`E0852`). A grandchild is the child's own
business.

When nvs itself is killed, a job object ends the child on Windows, and on Linux and macOS a watcher
process that nvs starts beside its first child does. Two children outlive a killed nvs there: one
running a set-user-ID program, which the watcher may have no permission to kill, and one started
before the watcher itself was killed. nvs finds a killed watcher at its next child start, writes a
`warn` record to the log and starts another.

A child started inside a `Core\Task` child task is ended and reported when that task ends. Under
`nvs run`, that report does not change the exit status. A handle names its child only in the task that
spawned it: a handle a child task returns names no child of the task that receives it, and any member
called on it there is a `FATAL`.

Both members take the same options bag, `rule:core-classes/process-options`, and its `timeout` bounds
every handle member that parks.
