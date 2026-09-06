`Core\Test::assertMatchesInline($value, "...")` holds the expectation in the source file, and
`nvs test --update` splices the produced value into that literal. Inline rather than a separate
snapshot file, because the failure mode of snapshot testing is a reviewer approving a diff they did
not read, and a diff inside the test body is one they will.

The updater is the one thing in `nvs test` that writes to a source file, and what it writes is the
expected literal and nothing else. The literal's span comes from a compile-time row per **written**
call, joined to the run by the test the mismatch happened in — the method is load-bearing, because
this workflow starts every snapshot empty and two of them would otherwise share one key. Where that
join is not a single site, nothing is written and the run names it: a rendering placed under a
snapshot nobody asserted is worse than the failing test it replaced.

What is written is a single-quoted literal, so a multi-line rendering stays multi-line and reads as
a diff. A run that rewrote a snapshot still reports the test as failed; the re-run is what says the
new text is the one the author meant. The rendering is canonical, ordered, and redacts a `secret`,
so a snapshot cannot become the place secrets get committed.
