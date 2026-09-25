- **A transferred call argument is released by the landing block, not by the normal edge.**
  `release_temporaries_since` skips a `TemporaryKind::Transferred` entry, so a lowering test
  asserting "the transferred value is never released" fails on the error path, where the frame still
  owes it: a callee that returned non-OK never took the reference. Assert per block — the call's own
  block for what the normal edge does, `inst.on_error`'s for what the throw does.
  [until: gone crates/nvs-ir/src/lower/call.rs:TemporaryKind::Transferred]
