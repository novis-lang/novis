- **`jiff`'s disambiguation does not spell the spring-forward rule, and the nearest option is wrong
  by half an hour.** `AmbiguousZoned::compatible()` shifts a civil time inside a gap forward by the
  gap's length and `earlier()` shifts it back, while
  `rule:config/a-missed-fire-is-skipped-and-a-dst-edge-fires-once` wants the transition itself, and
  a case asserting only the day passes against both. Read the missing minute with the gap's `after`
  offset and take `TimeZone::following(that).next()`, as `crates/nvs-config/src/schedule.rs`'s `at`
  does; only the fall-back half is a library call. [until: gone crates/nvs-config/src/schedule.rs:following]
