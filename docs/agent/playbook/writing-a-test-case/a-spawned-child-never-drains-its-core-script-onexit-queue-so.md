- **A spawned child never drains its `Core\Script::onExit` queue, so no `.nvst` case can put two
  exit-queue endings in one file.** `nvs_stdlib::script::run_exit_hooks` is called only from
  `crates/nvs-cli/src/main.rs`'s top-level script frame, so a `spawn script` child ends `ok=true`
  with its hooks never run and no diagnostic. A bound asserted on both sides over that table needs
  two endings and a script has one; whether a spawned child should drain is open, since the rule
  says "at most once per script". [until: gone crates/nvs-cli/src/main.rs:nvs_stdlib::script::run_exit_hooks]
