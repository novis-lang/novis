- **`live_bytes()` cannot see an allocation freed again inside the call under test, so a
  `live_bytes` delta passes a no-allocation guard either way.** `nvs_array_set` renders an `NvsStr`,
  hands it to the packed arm and drops it before returning — live delta zero, exactly like the pair
  that never allocated. Use `counting_alloc::allocated_bytes()`, the monotone total, for any claim
  about a *transient* cost, and assert in the same test that the old spelling *does* allocate, or a
  broken counter reads as a passing guard. [until: gone crates/nvs-runtime/src/budget.rs:live_bytes]
