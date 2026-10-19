- **Record a perf figure after `looper verify`, never before it.** Verify's formatter rewrote
  `crates/nvs-stdlib/src/xml.rs` after its figures were recorded, so every row's `impl_hash` missed
  the committed file and `bun nv proofs --gate` named 22 of them stale after the commit. Verify
  says "its files changed while it ran" when this happens. Run `bun nv proofs --record-perf` and
  then `bun nv proofs --gate` after the last verify and after any later edit to a hashed file, and
  commit only once the gate owes nothing. `until: gone tools/looper/src/verify/verify.ts:changed while it ran`
