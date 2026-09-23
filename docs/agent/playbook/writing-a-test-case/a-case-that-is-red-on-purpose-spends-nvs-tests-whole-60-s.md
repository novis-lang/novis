- **A case that is red on purpose spends `nvs-test`'s whole 60 s `CASE_TIMEOUT`, and an unbounded
  growth loop spends it on the host's memory.** `while (true) { $a .= $a; }` under a 16 MiB ceiling
  neither breaches nor aborts inside that timeout: it doubles into the machine's RAM and is killed
  on the clock. Bound such a loop just past the ceiling it must be stopped at — twenty-four
  doublings of five bytes, in
  `tests/conformance/error/a-loop-that-calls-nothing-is-stopped-by-the-memory-ceiling.nvst` — and
  leave a pure-CPU spin unbounded, since only the first takes the host with it.
  [until: reviewed 2026-09-11]
