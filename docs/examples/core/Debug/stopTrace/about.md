Stops the trace that `Core\Debug::startTrace` started, and closes its file.

After `Core\Debug::stopTrace`, the trace file is complete, and your program can read it. A call
that returns after the stop adds no line. A call that is still running when the trace stops also
has no line, even when it returns later.

When no trace is running, `stopTrace` does nothing. So you can call it in a `finally` block, and
the trace stops even when a step throws an error. `stopTrace` throws an error only when Novis could
not write every call to the file, for example because the disk is full.

**The examples below** trace one step of a job, stop a trace in a `finally` block, and show which
calls a trace has when it stops inside a method.
