- **A `??` whose right side is a plain integer literal answers `mixed` when the left side is a
  `?uint`, and the diagnostic underlines the whole expression rather than the literal.**
  `Core\Bytes::indexOf(...) ?? 1` was refused as `E0401: expected 'uint', found 'mixed'` inside a
  `uint` sum, because the literal is an `int` and the two sides have no common type. Write the
  literal at the receiver's own type — `?? (1 as uint)` — or cast the whole `??` expression, which
  is what the `bool`-returning benches beside it already do. [until: reviewed 2026-09-20]
