- **A `#[global_allocator]` declared in a library crate is only picked up by a binary that actually
  links that crate.** `nvs_runtime::alloc::Pooled` is registered from `nvs-runtime`'s own `lib.rs`,
  but rustc links an `--extern` crate lazily, so a new test binary whose sources never name
  `nvs_runtime` silently measures the platform heap. Name the crate in any binary that measures
  allocation; `benches/abi-probe/tests/perf_guards.rs`'s
  `an_allocation_round_trip_stays_in_the_pooled_cost_class` fails loudly in exactly that case, its
  cache count never moving. [until: gone benches/abi-probe/tests/perf_guards.rs:an_allocation_round_trip_stays_in_the_pooled_cost_class]
