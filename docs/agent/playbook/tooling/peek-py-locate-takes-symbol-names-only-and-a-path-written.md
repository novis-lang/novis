- **`peek.py --locate` takes symbol names only, and a path written after one is read as another
  symbol to look up.** `--locate at members crates/nvs-lsp/src/completion.rs` searched the whole
  tree for a symbol spelled like that path, and answered `at` with 250 definitions from every
  crate — the scoping the call looked like it had was never there. Scope with a `re:` target
  instead (`"crates/nvs-lsp/src/completion.rs:re:^fn "`), and put `--context N` ahead of every
  positional target, because a flag written between two of them is an argparse error rather than a
  flag applying to the rest. [until: reviewed 2026-09-11]
