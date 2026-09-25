- **A `Core\Time\TimeOfDay`'s four slots are not readable as properties, and the bottom of each
  field is a *checker* refusal, not a runtime one.** `$t->hour` is `E0405: `Core\Time\TimeOfDay` has
  no property named `hour``, so a case reads a field back out of `format` (`$t->format("HH") as
  int`, `format("SSSSSSSSS")` for nanoseconds). `at`'s parameters are `uint`, so `at(-1, 0)` is
  `E0401` and never reaches the member: only the top bound has a runtime half.
  [until: gone crates/nvs-stdlib/src/time.rs:Core\Time\TimeOfDay]
