The whole suite runs inside one task, so a test's isolate is a child of it and a `Core\Task::all`
written in a test has a calling task to put its own children under. The runner reads that tree at
the one moment it means anything — when the test's body returns, on the test's own stack — and **a
task still running then fails the test, named as such**. The alternative is what a scheduler does on
its own: cancel the leftovers as the task retires and report a green test that never waited for its
work.

**Under `#[Test(at:)]` a sleep is a virtual deadline.** A `Duration` sleep or a timer of the test's
task tree parks on a deadline read from the fixed clock
(`rule:testing/determinism-declared-on-the-test`), not from the operating system's. When no task of
the tree can run, the clock moves to the earliest deadline and wakes the tasks parked on it; tasks
whose deadlines are equal wake in the order they parked. So a sleep ends at once and moves
`Core\Time::now()` by its length, two sleeping tasks wake in deadline order, and a timeout races a
sleep the way it would in real time. That makes retry, backoff and timeout logic — some of the most
error-prone code anyone writes, and the least tested — testable in microseconds rather than in the
seconds it describes. `Core\Test::assertCompletes(fn, {within: Duration})` states a budget against
that same clock. Without `at:`, a sleep waits in real time.

**A request or a run a test makes has its own copy of the clock and its own deadlines**, taken when
the call starts (`rule:testing/in-process-request`). Its sleeps move its copy and never the test's.

**The expiries Novis computes itself read the same clock**: a cache entry's TTL and a queue job's
delay, so `Core\Test::advance` expires the one and makes the other due. A TTL kept by Redis or a
database, a lock wait and an I/O timeout stay in real time, because the other side of each is a real
program the test does not control. Cost: one check of the fixed clock per sleep and per expiry read,
and nothing where no clock is fixed.
