- **`nvs_stdlib_reaches_the_os_only_through_the_gate` is textual, so naming a forbidden type is as
  fatal as calling it.** The list is spellings (`std::fs`, `std::process::Command`, `std::env::var`)
  matched against the source above the file's `#[cfg(test)]` marker, so `fn write_chunk(file: &mut
  std::fs::File, ...)` and a `use std::fs::File;` both fail a gate about effects on a signature that
  performs none. Make the helper generic over `W: Write`, or infer the type from the door that
  answered it.
  [until: gone crates/nvs-stdlib/tests/capability.rs:reaches_the_os_only_through_the_gate]
