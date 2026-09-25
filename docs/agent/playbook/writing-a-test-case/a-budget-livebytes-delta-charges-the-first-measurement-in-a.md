- **A `budget::live_bytes` delta charges the first measurement in a test binary for what the core
  builds once and keeps.** The leaked per-core descriptor table lands inside the first member
  call's window — tens of kilobytes of fixed cost — so a small input and a large one measure
  nearly alike and no proportionality assertion holds. Measure a throwaway input first and drop
  the reading; the counters are thread-local, so nothing another test does beside it is in the
  number. [until: gone crates/nvs-codegen/tests/closures.rs:fn live_bytes_of_run]
