A test passes or fails the same way on a fast machine and on a slow one. A runner under load, an
aarch64 or a macOS runner can run the same code several times slower than the machine the test was
written on, and a bound taken from that machine fails there. So a test that has anything to say about
time says it by one of four patterns, and never by comparing a measured span with a constant.

1. **The order of events.** The test checks that one thing happened before another: "the limit handler
   ran, then the next request was served". It waits for the event itself: a frame, a line, a call, a
   signal. A time bound it still carries is only a hang guard, which fails a test whose event never
   comes, and it is as large as the hostile timeout.
2. **Two readings of the same clock.** A test that checks how much time was charged compares the charge
   with a reading of the same clock, taken by the test on the same thread. It never compares the charge
   with a constant such as 20 ms. CPU time and wall time are different clocks: a thread can be off its
   CPU for most of a wall-clock span, so a CPU charge is never checked against a wall-clock reading.
3. **A clock the test controls.** Where the code under test already reads time through a seam, the test
   drives that seam: `crate::slice`'s `Clock` in `nvs-host` is one. No clock abstraction is added only
   for a test, because a direct reference is simpler than a mapping layer every reader has to follow. A
   test that would need a seam where none exists uses the first or second pattern.
4. **A bound measured in the same run.** Where the property itself is about time, such as how far past
   its limit a stopped program ran, the bound is what the machine measured during that run plus the
   code's own fixed term. For a time-limit attack, the proofs runner explains a stop's overshoot by the
   watchdog's sweep interval, how late the sweeps between the crossing and the stop woke, and one poll
   gap of the code that was running (`timeOvershoot` in `tools/nv/proofs/run.ts`). An overshoot those do
   not explain is red on every machine. The poll gap is the code's term, `nvs_runtime::POLL_GAP`: a
   helper's batch is sized so its polls come well inside one sweep interval. Measuring each gap would
   put a clock read on the request path, which this rule declines below.

**A test drops a time bound only where the path it covers has a bench or a counted cost guard.** A
bound in a test is often the only thing that would notice a path getting slower. Before it is taken
out, the path gets a bench under `benches/` or a count in a cost guard
(`rule:testing/perf-two-mechanisms`), and the change that removes the bound names it.

A test that follows this rule still waits for its events, so it takes as long as the machine needs and
no longer. What it never does is decide its verdict by how fast that was.

- **Memory:** nothing on the request path. The fourth pattern records, on the watchdog's own thread, one
  duration per sweep in flight per core.
- **Cost:** a test that loses a bound may gain a bench or a cost guard, which adds CI minutes.

**Declined:**
- **Retries**, as nextest `retries` or `bun nv ci --rerun` used as a fix. They make the build green and
  hide a real stop that overshoots. `rule:testing/runner-is-strict` already says a retry is never green.
- **Running timing tests alone, or in a nextest test group.** It removes only the load the suite makes
  itself. A slow runner or a busy developer machine stays flaky.
- **A `--slow` mode** that pins tests to few cores or keeps the other cores busy. It finds flaky tests
  only some of the time, so it is itself flaky.
- **A fixed slack on a time stop, or raising any other fixed bound.** A larger constant hides a real
  regression on a fast machine and still fails on a machine slower than the one it was chosen on.
- **A CPU-clock read on the request thread at each poll**, to judge a stop exactly. Every CPU-clock read
  is a system call (`crates/nvs-host/src/cpuclock.rs`), and this one would sit on the request path.
- **A per-thread kernel CPU timer**, `timer_create` on `CLOCK_THREAD_CPUTIME_ID`. It exists on Linux
  alone, so macOS and Windows would run a second mechanism that CI's Linux jobs never test.
