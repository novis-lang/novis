- **`python tools/dossier.py --bless` rebuilds `target/release/nvs` before it runs anything, so a
  slice that edits Rust and then blesses pays a whole release build.** A feature-proof slice
  usually does both — the `.nvs` files under the three trees, and a `covers:` test in the member's
  own crate — and in that order every bless after the first waits minutes on a build whose only
  change is a `#[test]` the examples never reach. Write and bless a whole group's examples first,
  then add all of the group's Rust tests once, so the group pays one release build rather than one
  per member. [until: reviewed 2026-09-22]
