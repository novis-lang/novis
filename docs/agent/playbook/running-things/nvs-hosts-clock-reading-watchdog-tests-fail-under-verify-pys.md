- **`nvs-host`'s clock-reading watchdog tests fail under `nv verify`'s side-by-side test run and
  pass alone, so the gate goes red on a crate your slice never touched.**
  `a_published_request_carries_a_baseline_and_reads_its_trees_ceiling` and
  `a_run_that_is_no_core_is_sampled_and_reported_never` in `crates/nvs-host/src/watchdog.rs` measure
  process CPU time every other test binary competes for, and the wedged-worker test's margin too.
  Read the failing name before looking for your own change in it; a rerun only moves it. [until: gone crates/nvs-host/src/watchdog.rs:a_published_request_carries_a_baseline_and_reads_its_trees_ceiling]
