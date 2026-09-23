- **A `spawn script` option that type-checks is not necessarily carried anywhere.**
  `nvs_types::expr::isolate` accepts and types all five options, but
  `Lowering::lower_spawn_script` builds the `CoreCall` out of four values — the entry, `args:`,
  `output:` and `on:` — so `limits:` and `grants:` are checked and then dropped, and a program that
  writes a narrowing gets none of it. Read the lowering's `written(SpawnOptionKey::…)` arms before
  writing a case over an option, because a `.nvst` that asserts a narrowing took effect will fail for
  a reason no diagnostic names.
  [until: gone crates/nvs-ir/src/lower/expr.rs:SpawnOptionKey::Limits]
