- **A `loop-goal.toml` check can be filed in the crate whose *name* matches the subcommand, and the
  subcommand lives in `nvs-cli`.** `nvs serve` is a subcommand of the `nvs` binary
  (`crates/nvs-cli/src/serve.rs`); `nvs-server` is the accept-loop library it hands a bound socket
  to, resolves no configuration and boots nothing, and the manifest and `[lints]` triages both pass
  because `nvs-cli` depends on it. Triage a check naming a `nvs <verb>` behaviour by `grep -n
  'Command::' crates/nvs-cli/src/main.rs`, which lists every subcommand beside the module that
  answers it. [until: gone tools/nv/cmd/orient.ts:a loop-goal.toml*]
