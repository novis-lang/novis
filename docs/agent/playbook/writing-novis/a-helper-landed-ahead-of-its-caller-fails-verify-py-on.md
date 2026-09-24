- **A helper landed ahead of its caller fails `nv verify` on `dead_code`; `#[expect(dead_code,
  reason = "…")]` is the marker that removes itself.** `cargo clippy --all-targets -- -D warnings`
  stops the build on an uncalled private function; `#[allow]` goes stale silently while `#[expect]`
  fails the day the function is used. Do not pair it with a `#[cfg(test)]` test of the same function
  — the test build makes the expectation unfulfilled, itself a warning — and spell it
  `#[cfg_attr(not(test), expect(dead_code, reason = "…"))]` when the tests are the only reader.
  [until: reviewed 2026-09-06]
