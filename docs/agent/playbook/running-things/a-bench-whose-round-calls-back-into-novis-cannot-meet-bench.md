- **A bench whose round calls back into Novis cannot meet `// bench: allocations 0`, and
  `--record-perf` refuses the figure rather than recording it.** `nvs_runtime::call_closure` builds
  each call's argument slots in a heap vector (`crates/nvs-runtime/src/lib.rs` `# known gap `nvs-runtime/call-through-a-callable-builds-its`), so
  a fold over three ints with nothing of its own on the heap still measures 5 allocations per op.
  Declare `allocations 0` only where the round enters no closure — `count` does and `map`, `filter`
  and `reduce` do not — and read the measured number as the member's cost plus one vector per
  callback. [until: reviewed 2026-09-20]
