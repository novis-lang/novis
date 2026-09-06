What crosses a thread boundary is a task id and a poke of one core's poller, and nothing else — no
reference to a scheduler, a stack or an arena. Firing it from any thread queues that id on **that**
core and ends the poll the core is asleep in; the scheduler there moves the task from parked to ready
and resumes it.

The re-poll therefore happens on the core that parked, on the coroutine's own stack, under the same
thread-local reactor. **A task never migrates**, so "the future woke on a different core" is not a
state this design has. The only object ever on two cores at once is the permission, and it carries an
id.

It is also why a waker's payload is a shared flag and slot rather than the scheduler's own same-core
wake, which is deliberately not thread-safe: a waker built over that would be sound exactly until a
clone crossed a thread, and unsound with no diagnostic afterwards.
