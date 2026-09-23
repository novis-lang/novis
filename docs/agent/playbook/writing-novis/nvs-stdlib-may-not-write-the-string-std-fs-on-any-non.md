- **`nvs-stdlib` may not write the string `std::fs` on any non-comment line, a type name included.**
  `nvs_stdlib_reaches_the_os_only_through_the_gate` is a literal substring scan with no allowlist,
  so `&std::fs::Metadata` or `std::fs::TryLockError::WouldBlock` is reported as "reaches the
  operating system directly" about a line that reaches nothing. Spell the type
  `nvs_runtime::capability::Metadata`, convert a `TryLockError` through `std::io::Error`'s
  `ErrorKind::WouldBlock`, and keep `std::fs` to `//` lines.
  [until: gone crates/nvs-stdlib/tests/capability.rs:nvs_stdlib_reaches_the_os_only_through]
