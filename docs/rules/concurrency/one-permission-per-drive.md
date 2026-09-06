A wake permission is taken per **drive**, not per park. The reactor raises its outstanding count when
it issues a handle and lowers it only when the id that handle queued is drained on the core, so every
handle issued is a wake delivered — by the wake itself, or by the drop that stands in for it. A
handle taken freshly for each park would poke the core once per readiness edge with a wake nobody
asked for.

So the loop installs one permission and leaves it installed across parks; the waker **takes** it out
of the slot when it fires; the next park finds the slot empty and issues a fresh one.

A connection whose every wait ends in socket readiness — the ordinary one, since the reactor wakes a
task by id and never through a waker — therefore issues exactly one permission for the whole
connection, and delivers it when the drive returns.
