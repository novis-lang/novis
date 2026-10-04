- **`nvs-lsp` gaining a `nvs-types` dependency fails `no_crate_the_server_links_writes_to_stdout`,
  and every line named is in `nvs-runtime`.** The type checker links `nvs-stdlib`, which links the
  runtime, whose `OutputSink::Stdout` is how a program's `echo` reaches a terminal, so the guard's
  `stdout()` pattern fires two hops past the crate you added and names neither. The exemption is in
  that test as `THE_OUTPUT_SINK`, paired with `nothing_under_the_server_wires_a_program_to_stdout`;
  widen the exemption again and check both rather than dropping the pattern.
  [until: gone crates/nvs-lsp/tests/stdout_policy.rs:const THE_OUTPUT_SINK]
