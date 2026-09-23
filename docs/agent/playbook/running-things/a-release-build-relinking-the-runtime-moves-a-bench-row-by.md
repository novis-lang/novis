- **A release build relinking the runtime moves a bench row by several percent, with no code
  change.** Two builds whose member was byte-identical read differently, and rows nothing touched
  moved with them. An A/B on a single row is only worth reading when the delta is well past that
  noise; quote the median, and re-run the base binary once before believing a small regression.
  [until: reviewed 2026-09-06]
