- **A test that reads `nvs_codegen::disassemble` is x86_64-only until it says which backend it
  means.** Cranelift prints the vcode of whichever backend it emitted for, so a scan for lines
  beginning `call ` counts zero on aarch64 (`bl 0`, `blr <reg>`) — a structurally empty answer that
  a "no more calls than sites" guard passes while looking at nothing. Anything walking calls or
  branches out of the disassembly wants a `target_arch` gate, as `perf_guards.rs`'s `path_calls`
  has; macos-aarch64 is in the test matrix.
  [until: gone benches/abi-probe/tests/perf_guards.rs:path_calls]
