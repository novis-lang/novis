- **`Timers::publish` bumps a deadline equal to its base by a nanosecond, and two adjacent
  `Instant::now()` calls can be equal.** A test that arms at `let start = Instant::now()` on a
  `Timers` built the line before and sweeps at exactly `start + margin` is a nanosecond short of
  overdue and reports nothing, only under load. Arm strictly after the base whenever the sweep
  instant derives from the armed one. [until: gone crates/nvs-host/src/timer.rs:.max(NOTHING + 1)]
