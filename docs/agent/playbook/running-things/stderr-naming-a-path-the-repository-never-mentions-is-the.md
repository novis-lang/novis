- **Stderr naming a path the repository never mentions is the shared test database talking; the exit
  code beside it is the finding.** `nvs.toml`'s `[queue] workers = 1` makes every `nvs run` drain
  every `novis_test` queue, so a job `crates/nvs-stdlib/tests/queue.rs` pushed prints as an
  unrelated `could not read <path>`. Read the run's block in `.loop/logs/<run>-console.log`: `exit
  3221226505` is `0xC0000409`, Rust's abort on Windows, and its panic is only there.
  [until: reviewed 2026-09-06]
