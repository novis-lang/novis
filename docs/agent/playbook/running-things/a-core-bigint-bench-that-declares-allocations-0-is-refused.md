- **A `Core\BigInt` bench that declares `allocations 0` is refused, because every member of that
  class materializes its receiver.** `crates/nvs-stdlib/src/bigint.rs`'s `operand` rebuilds the
  magnitude from the instance's two slots on each call, so even a member answering an `int`
  allocates once, and `--record-perf` writes no record while the declaration disagrees with the
  count. Read what a sibling member already recorded in `docs/perf/members.ndjson` before choosing
  the number a new bench declares. [until: reviewed 2026-09-20]
