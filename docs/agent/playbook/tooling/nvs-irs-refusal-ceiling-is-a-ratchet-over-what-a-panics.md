- **`nvs-ir`'s refusal ceiling is a ratchet over what a panic's *message says*, so rewording one can
  turn `nv verify` red with no new site.** `tools/nv/cmd/holes.ts`'s `REFUSAL` regex matches `does
  not (yet) lower`, `only lowers`, `has no arm for` and their siblings inside any `panic!`/`assert!`
  literal, so adding "which this crate does not lower yet" to a consistency panic raises the count.
  The tell is a red `cargo test --test refusals` while `bun nv holes` reports `UNATTRIBUTED: 0`; a
  claim of a lowering gap goes in the crate docs' known-gaps section, and the panic keeps what it
  said. [until: gone tools/nv/cmd/holes.ts:REFUSAL]
