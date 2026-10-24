- **A rewritten `Core` reference card is quoted word for word outside the registry.** A hover
  `.lspt` under `tests/lsp/hover/`, the stub golden `the_method_enum_is_spelled_exactly_so` in
  `crates/nvs-lsp/src/stubs.rs`, the `nvs meta --json` goldens in `crates/nvs-cli/tests/meta.rs` and
  `docs/reference/tools/50-agents.md` all freeze card text, and none of them is rendered. Before
  committing a card sweep, search the tree for a phrase of each removed card line, outside
  `crates/nvs-stdlib/src` and `website/`, and run `nvs lsp-test tests/lsp/hover/`. See
  `crates/nvs-stdlib/src/router.rs`. `until: gone crates/nvs-lsp/src/stubs.rs:the_method_enum_is_spelled_exactly_so`
