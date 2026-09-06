The whole suite runs inside one task, so a test's isolate is a child of it and a `Core\Task::all`
written in a test has a calling task to put its own children under. The runner reads that tree at
the one moment it means anything — when the test's body returns, on the test's own stack — and **a
task still running then fails the test, named as such**. The alternative is what a scheduler does on
its own: cancel the leftovers as the task retires and report a green test that never waited for its
work.

Because the clock is under the test's control, a `Duration` sleep inside a task elapses instantly.
That makes retry, backoff and timeout logic — some of the most error-prone code anyone writes, and
the least tested — testable in microseconds rather than in the seconds it describes.
