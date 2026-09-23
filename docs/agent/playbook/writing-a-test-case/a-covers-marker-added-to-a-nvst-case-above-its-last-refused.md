- **A `covers:` marker added to a `.nvst` case above its last refused line turns a green case red.**
  An `--EXPECTF-ERROR--` block's `-->` anchors are absolute line numbers inside the `--FILE--`
  program, so one comment line added at the top moves every one of them, and the failure then reads
  as a diagnostic that changed rather than as a line that moved. Put the marker after the last
  statement the block quotes — the end of the `--FILE--` block is always safe — and run
  `target/debug/nvs test <case>.nvst` before committing it.
  [until: reviewed 2026-09-19]
