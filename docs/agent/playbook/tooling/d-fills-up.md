- **`D:` fills up.** `target/debug` grows to tens of GB and `cargo test` then fails as a wall of
  `link.exe` errors whose real message (`no space on device`) only appears without a `Select-String`
  filter. Check `Get-PSDrive D` before diagnosing a linker failure; `cargo clean` frees it in
  seconds and the rebuild is a few minutes. [until: gone AGENTS.md:filled the disk]
