A wake permission is taken per **drive**, not per park. The reactor raises its outstanding count when
it issues a handle and lowers it on the core: when the id that handle queued is drained, or on the spot
when the task it names gives the handle up (`rule:concurrency/a-handle-given-up-by-its-task-wakes-nothing`).
So every handle issued is collected there, and a handle taken freshly for each park would be issued and
collected once per readiness edge for a wake nobody asked for.

So the loop installs one permission and leaves it installed across parks; the waker **takes** it out
of the slot when it fires; the next park finds the slot empty and issues a fresh one.

A connection whose every wait ends in socket readiness — the ordinary one, since the reactor wakes a
task by id and never through a waker — therefore issues exactly one permission for the whole
connection, and gives it back unused when the drive returns.
