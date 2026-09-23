- **A test that arms a wall-clock deadline before it starts the scheduler charges the fixture's own
  setup to that deadline, and then fails under a sanitizer alone.**
  `a_timer_and_a_deadline_are_the_same_wheel` armed a socket 10ms out and afterwards bound a listener,
  installed a reactor and built a scheduler, so under `tools/tsan.sh` the read answered `TimedOut`
  without ever parking and `parked` came back 1 where the test wanted 2 — green on Windows, red on the
  floor, with nothing in the diff to blame. Arm a deadline inside the task that waits on it, where the
  window is the wait's own, and leave a margin the slowest leg can spend.
  [until: reviewed 2026-09-20]
