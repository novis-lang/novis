- **`every_error_path_is_asserted_or_declared_unreachable` reads its declaration phrase only within
  `DECLARATION_WINDOW` lines above the `Fault::` line, so a long comment with the phrase at the top
  declares nothing.** `conformance_coverage.rs`'s scan walks upward from the site and stops at the
  first line holding `Fault::`, and the failure re-prints the worklist line with no hint that the
  comment exists. Put `unreachable from source` in the comment's last sentence, and give a helper
  with several guards one declaration per guard rather than one at the top of the function.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:DECLARATION_WINDOW]
