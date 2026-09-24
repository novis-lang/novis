- **Making a type `pub` owes it a `#[derive(Debug)]`.** The workspace denies
  `missing_debug_implementations`, so promoting a private struct to the public API compiles and then
  fails at clippy, after the tests have already run. Add the derive in the same edit as the `pub`,
  not after `nv verify` says so. [until: gone Cargo.toml:missing_debug_implementations]
