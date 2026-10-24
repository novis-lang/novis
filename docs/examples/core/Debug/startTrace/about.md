Starts writing a record of every call your program makes to a file, until `Core\Debug::stopTrace`.

When a call returns, `Core\Debug::startTrace` adds one line to the file. Each line is one JSON
object. `callee` is the name of the method or function, and `status` is `ok`, or `thrown` for a call
that threw an error. `start` and `end` are clock readings in nanoseconds, so their difference is how
long the call took. A call that is still running when the trace stops has no line.

The configuration must allow a trace. `[debug] mode` must include `"trace"`, and
`[capabilities.debug] trace` must include the folder of the file. Permission to write files is not
enough. Otherwise `startTrace` throws a `RuntimeError`. A second `startTrace` before the stop throws
a `LogicError`.

**The examples below** list the methods a checkout calls, count how often one method runs, and
show where a trace may go and that only one trace runs at a time.
