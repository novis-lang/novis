- **A `cranelift-jit` panic in `compiled_blob.rs` (`TryFromIntError(NegOverflow)`) is address
  layout, not the tree; `-- --test-threads=1` settles it.** It is a PC-relative relocation whose
  target landed over 2 GB from the JIT buffer, which `-p nvs-cli`'s
  `ten_thousand_concurrent_cold_requests_compile_the_file_exactly_once` provokes when it shares
  address space with other test binaries. Re-run alone before touching a line.
  [until: reviewed 2026-09-06]
