- **A `nvs-cli` bin test that installs the process's service manager records every other case's
  stop.** `crate::service::Notify::install` sets a `OnceLock` the first caller owns for the
  binary's life, and `crate::stop::deliver_to` reports `STOPPING=1` into it, so a case asserting
  its own recorder reads a line another case wrote — under load only. Take
  `crate::stop::ONE_STOP_AT_A_TIME` in any case that reaches a stop, including the ones arriving
  through the SCM controls, which call it without naming it. [until: gone crates/nvs-cli/src/stop.rs:ONE_STOP_AT_A_TIME]
