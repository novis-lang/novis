- **A `-p nvs-cli` test whose program `spawn script`s needs a scheduler and a reactor;
  `nvs_host::Isolate::run` alone installs neither.** Neither failure names it: `script.spawn` `is
  not granted`, then `needs a scheduler on this thread`. Copy `crates/nvs-cli/src/script.rs`'s
  `run_serving`: `Scheduler::new()`, `spawn(granting_ctx(), TaskRoot::Request, …)`,
  `reactor::install(Reactor::new()?)`, `run_until_idle`. [until: reviewed 2026-09-06]
