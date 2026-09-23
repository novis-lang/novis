- **A difference of two `nvs_runtime::budget::live_bytes()` readings is never exactly the payload,
  and the shortfall belongs to whoever reads second.** The counter is a thread-local fed by the
  global allocator, so it counts everything live on the thread, the fixture's own copy included.
  Name an allowance for the second reader's footprint (64 KiB against a 2 MiB signal still catches a
  retained arena) rather than an exact number. [until: reviewed 2026-09-06]
