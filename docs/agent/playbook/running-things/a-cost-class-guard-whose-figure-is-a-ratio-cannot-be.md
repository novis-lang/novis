- **A cost-class guard whose figure is a *ratio* cannot be repaired by fixing the statistic.** The
  fan-out guard in `benches/abi-probe/tests/perf_guards.rs` already takes the minimum of five
  interleaved rounds, and a box with no free cores still collapses it from 3.86x to 0.08x — the same
  reading a picker that stopped spreading gives. Measure what the machine can give at that moment
  too, the same children on plain threads started once per batch, and report the guard not measured
  rather than failed when that figure is under the floor.
  [until: gone benches/abi-probe/tests/perf_guards.rs:plain_thread_fan_out]
