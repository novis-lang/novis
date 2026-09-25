- **When `run_until_idle` returns with tasks still parked, suspect the turn's "nothing woke" exit
  before a lost wake.** One drain takes the queued ids of *several* pokes at once, so surplus pokes
  come back ready over an empty queue, and `Reactor::turn` reporting `0` woken reads as idle. A test
  that runs wide is worth writing over landed behaviour;
  `a_blocking_call_goes_to_a_pool_bounded_at_twice_the_core_count` in
  `crates/nvs-host/src/blocking.rs` is the example. [until: gone crates/nvs-host/src/blocking.rs:a_blocking_call_goes_to_a_pool_bounded_at_twice_the_core_count]
