- **A sanitizer changing an answer does not mean the sanitizer is involved.** ASAN turned an
  out-of-bounds slot read into `Some(0)` on the asan leg alone, which read as an ASAN-specific ABI
  fault; it was an ordinary off-by-one that ASAN merely made deterministic by poisoning the slot
  past the array. Before theorising about instrumentation, print what the callee actually received
  for several distinct non-zero arguments. [until: reviewed 2026-09-06]
