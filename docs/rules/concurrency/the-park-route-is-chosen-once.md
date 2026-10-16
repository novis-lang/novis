Where a drive parks is decided once, when it starts, because a task never migrates and so the answer
cannot change underneath it. There are three routes and no fourth.

On a core with a reactor — the server — a park suspends the task, and the wake is the installed
permission. On a core with no reactor, which is a scheduler-only test, there is nothing to arrange a
wake with, so the drive yields to the back of the run queue and polls again: spinning through the run
queue is deliberate, because a park nothing can end is a wedge. Off a core entirely — a `nvs run`
program, a unit test — the thread itself parks and is unparked.

Blocking the thread is right in that last case for the reason every off-core wait already gives:
there is no coroutine to suspend and no neighbour to starve, so blocking starves nobody. On a core it
would be precisely wrong.
