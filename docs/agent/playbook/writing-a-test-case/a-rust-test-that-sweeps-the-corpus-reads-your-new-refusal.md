- **A Rust test that sweeps the corpus reads your new refusal cases as corpus, and the failure lands
  in a crate you did not touch.** `tests/conformance/` holds cases whose whole subject is a
  diagnostic, so a sweep asserting "no corpus file reports `E0nnn`" fails the moment someone pins
  `E0nnn` — the report named a `-p nvs-syntax` test while the edit was six `.nvst` files. Such a
  sweep has to drop a case carrying `--EXPECT-ERROR--` or `--EXPECTF-ERROR--`, which states a
  diagnostic rather than carrying one. [until: reviewed 2026-09-07]
