- **Refusing a construct that already parses breaks the tests that used it as a fixture, not as a
  subject.** `<>` was a row in `operators_longest_match_wins`, and `new class { … }` was the body
  two `casing.rs` fixtures used to prove the casing pass descends into a nested declaration; all
  three called a helper asserting a clean parse, so they failed with the new code rather than with
  anything about the shape they test. Before writing a refusal, grep the crate for the spelling;
  move the fixture to the helper that collects both halves (`casing.rs`'s `parse_and_check`) or hand
  the shape to the new test, never weaken the refusal. [until: reviewed 2026-09-06]
