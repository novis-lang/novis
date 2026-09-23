- **Widening a numeric literal's placement is a union question before it is a type question, and
  `tests/differential/` is what tells you.** Passing a `decimal` expectation through `-e` with
  `placed_literal`'s walk also reached the `decimal` arm of every numeric union, so
  `Core\Math::abs(-0.0)` became a `decimal` printing `0.0` while `abs(0.0)` stayed a `float`
  printing `0`; build and `cargo test` were both green and only the PHP twin caught it. Place `-e`
  with the same exact test the bare literal's arm uses (`wants_decimal`, not `placed_literal`), and
  run `nvs test tests/differential` before believing a placement change is contained.
  [until: reviewed 2026-09-22]
