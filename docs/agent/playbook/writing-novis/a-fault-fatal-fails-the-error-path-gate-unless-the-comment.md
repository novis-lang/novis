- **A `Fault::fatal` fails the error-path gate unless the comment above it holds the literal words
  *unreachable from source*.** `conformance_coverage.rs`'s gate greps the eight lines above the site
  for that phrase, not for a reason, so a comment arguing the case in its own words fails as though
  the member owed a `.nvst` case for an unreachable path. Use the phrase; `Core\Str::length`'s
  `u64::try_from` arm carries the wording for a site no diagnostic refuses.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:every_error_path_is_asserted]
