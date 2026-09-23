- **`nvs_host::sleep` re-arms past a wake on purpose; waiting on a peer needs
  `nvs_host::timer::wait_until`, which is not re-exported.** `park_until` loops until its deadline
  because the caller asked for an instant, so a `parent.wake()` delivered to a sleeping loop is
  swallowed and the whole bound is paid every time, with nothing failing. `grep -n "pub fn"
  crates/nvs-host/src/timer.rs` is the list, and the same is true of every `pub mod` in that crate
  whose `pub use` line is a subset. [until: gone crates/nvs-host/src/timer.rs:pub fn wait_until]
