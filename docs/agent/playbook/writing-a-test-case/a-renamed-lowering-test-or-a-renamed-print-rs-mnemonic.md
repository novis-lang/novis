- **A renamed lowering test, or a renamed `print.rs` mnemonic, strands an `insta` snapshot.** The
  file is `crates/nvs-ir/src/lower/snapshots/nvs_ir__lower__tests__<fn>.snap`, and a mnemonic sits
  *inside* every snapshot whose fixture lowers it, so the failure reads as a lowering regression
  rather than as the rename it is. `git mv` the snapshot in the same slice and grep that directory
  for the old mnemonic. [until: gone crates/nvs-ir/src/lower/snapshots]
