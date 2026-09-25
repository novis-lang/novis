- **A serving core ends when `run_until_idle` reports `parked == 0`, so a task that parks forever on
  that scheduler wedges the shutdown.** `crates/nvs-cli/src/serve.rs:1229` loops until nothing is
  parked, so a receptionist parked on its bell there makes the count never reach zero and reads as a
  hung `nvs serve`. Give anything long-lived put on a serving core an ending of its own —
  `nvs_host::reactor::wake_at_drain` is the one this process already has.
  [until: gone crates/nvs-cli/src/serve.rs:run_until_idle]
