- **A `QName::to_string()` at the top of a per-call checker hook is an allocation per call site, and
  what catches it is `nvs-cli`'s 10k cold-compile guard failing inside cranelift.**
  `script::tests::ten_thousand_concurrent_cold_requests_compile_the_file_exactly_once` fails as a
  wrong count or a `TryFromIntError(NegOverflow)` panic in `cranelift-jit`, passes alone, and passes
  with the change stashed — `git stash push -- crates/<touched>` is the whole bisect. Test the
  `&str` half of the roster row first and stringify only after it matches.
  [until: gone crates/nvs-cli/src/script.rs:ten_thousand_concurrent_cold_requests_compile_the_file_exactly_once]
