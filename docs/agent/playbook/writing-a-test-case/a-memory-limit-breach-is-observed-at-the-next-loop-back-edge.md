- **A memory-limit breach is observed at the next loop back edge or the next member call, whichever
  the program reaches first, and an `$a[] = ...` on its own is neither.** The allocator raises
  `SafepointFlags::MEMORY_LIMIT` at the crossing, so a fixture that grows inside a `while` stops
  inside it; straight-line growth reaches nothing until `run_helper` asks ahead of a member's body.
  Put the loop or the member call where you want the breach reported, and expect nothing printed
  after the crossing. [until: gone crates/nvs-runtime/src/budget.rs:fn publish]
