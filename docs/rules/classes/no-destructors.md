Novis has no destructors. There is no refcount-triggered cleanup hook and no scope-exit hook, and
`__destruct` cannot even be declared. Cleanup a destructor would hold — closing a handle, releasing a
lock, flushing a buffer — becomes an explicit method the holder calls when it is actually done.

Two independent arguments each suffice. There is no sound place to report a throw: every call returns
a checked status to a caller, and a destructor has no call site — it fires from wherever a refcount
happens to reach zero, which is an assignment, a loop step, or a return that has nothing to do with
the failure. And it would undo the wholesale heap drop, whose whole point is not walking live objects
individually at request end.

One thing does run when a refcount reaches zero, and it is not a destructor: a generator suspended
inside a `try ... finally` is resumed in a return-like mode so the `finally` runs
(`rule:iteration/generators`). Nothing is declared, no name is recognized, and the release resumes a
frame the program had already entered. A throw escaping such a `finally` is discarded, since a release
is exactly the site with nowhere to report one.

What it costs is real: no RAII, so a caller who forgets an explicit `close()` gets nothing. A
destructor is an unreliable safety net, but it is a net.
