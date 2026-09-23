- **A ratio guard in `benches/abi-probe/tests/perf_guards.rs` measures the guards beside it: libtest
  runs a binary's tests on as many threads as the host has cores.**
  `a_cpu_bound_fan_out_across_four_worker_cores_is_near_linear_by_the_margin_this_test_names` inverts
  to under 1x in the full binary and is healthy alone, its placed half starved of the cores it fans
  out onto. Every guard there takes `serialised()` first and
  `every_guard_in_this_binary_takes_the_lock` counts the line, so write a new one holding it.
  [until: gone benches/abi-probe/tests/perf_guards.rs:fn serialised]
