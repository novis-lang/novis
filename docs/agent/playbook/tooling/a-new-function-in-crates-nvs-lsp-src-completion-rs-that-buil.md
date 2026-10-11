- **A new function in `crates/nvs-lsp/src/completion.rs` that builds a `CompletionItem` fails
  `every_completion_source_names_a_compiler_table` until it is listed.** The test reads the module's
  item-producing functions and compares them with the `SOURCED` table in
  `crates/nvs-lsp/tests/completion.rs`, which is a fixed-size array. Add one row per new function, naming
  the token its body reads, and raise the array's length in the same edit. `until: gone crates/nvs-lsp/tests/completion.rs:const SOURCED`
