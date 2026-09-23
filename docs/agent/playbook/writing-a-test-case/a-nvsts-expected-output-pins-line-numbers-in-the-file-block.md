- **A `.nvst`'s expected output pins line numbers in the `--FILE--` block above it, so a tree-wide
  script that touches case files must preserve their line count.** A rewrite that joined two `//`
  comment lines into one shifted every `--> case.nvs:NN:CC` below the join, and cases whose expected
  output is program text survive the same join, which is what makes it look safe when spot-checked.
  The budget is lines, not bytes: rewrite the token in place and leave the break standing rather
  than reflowing and re-recording. [until: reviewed 2026-09-06]
