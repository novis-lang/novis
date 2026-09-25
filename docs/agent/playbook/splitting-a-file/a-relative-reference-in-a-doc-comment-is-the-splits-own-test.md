- **A relative reference in a doc comment is the split's own test.** `grep -n
  "above\|below\|neighbour"` over the moved halves finds the sentences that stopped being true, and
  one hit in `ctx.rs` was not stale prose but a mis-seam: `set_inbound`'s "the inbound half of the
  channel the three methods above are the outbound half of" was the file saying it did not belong in
  `output.rs`. A comment that describes its own neighbours is the only place a wrong cut announces
  itself, because the compiler never will. [until: gone crates/nvs-runtime/src/ctx/inbound.rs:set_inbound]
