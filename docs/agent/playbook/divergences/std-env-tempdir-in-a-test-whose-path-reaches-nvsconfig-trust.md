- **`std::env::temp_dir()` in a test whose path reaches `nvs_config::trust::check` is refused on
  Unix.** `/tmp` is mode 1777, `rule:config/ownership-is-the-trust-boundary` refuses a group- or
  world-writable directory, and the check runs on the directory *and its parent* — so a tight
  scratch directory of your own is still refused, with a `Breach` naming a mode you did not set.
  Scratch beside the test binary instead (`std::env::current_exe()`'s parent, under `target/`);
  Windows hides this entirely because its temp dir is per-user. [until: gone crates/nvs-config/src/trust.rs:Breach]
