- **A `-p <crate>` test asserting a process-wide ceiling races every other test in the binary; make
  the count a parameter rather than serialise the tests.** A bound in one `static AtomicU64` is only
  assertable at a ceiling of one, so `Slot::take(1)` answers whichever other test held a connection
  at that instant. Add `take_from(count: &'static AtomicU64, ceiling)` with `take(ceiling)`
  delegating, and let the test own a `static COUNT`. [until: reviewed 2026-09-06]
