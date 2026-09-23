- **`continue 2` inside a `switch` is PHP's idiomatic spelling, so counting `continue N` over loops
  alone silently breaks ported code.** Skipping `switch` frames compiles and passes every test, then
  rejects `foreach { switch { case: continue 2; } }` and, with two nested loops, retargets PHP's
  inner loop to the outer one. Count every frame as PHP does, then walk outward from the frame the
  level lands on to the nearest loop;
  `tests/conformance/lang/a-break-leaves-the-level-it-names.nvst` pins it.
  [until: reviewed 2026-09-06]
