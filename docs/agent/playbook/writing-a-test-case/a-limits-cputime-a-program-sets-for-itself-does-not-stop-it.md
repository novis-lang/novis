- **A `limits.cpu_time` a program sets for itself does not stop it under `nvs run`.** The watchdog
  is registered once from the ceiling in force before the task starts, and `RunningRequest::new`
  answers `None` for a request under no cap (`crates/nvs-cli/src/main.rs:2459`), so a later
  `Core\Config::set` moves the number and nothing charges it. An attack that needs a CPU stop writes
  the ceiling into the `nvs.toml` the run reads; a memory ceiling set from inside *is* charged, and
  is the cheap way to reach a limit handler.
  [until: gone crates/nvs-cli/src/main.rs:let (view, cpu_limit) = ceiling]
