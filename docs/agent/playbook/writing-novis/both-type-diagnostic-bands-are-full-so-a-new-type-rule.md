- **Both type diagnostic bands are full, so a new type rule cannot have its own code.** `E04xx` and
  `E07xx` are full, `python tools/brief.py` prints both as `FULL` rather than a next number, and
  `E09xx` is internal compiler errors. Reuse an existing code whose message the rule can honestly
  rewrite, or write the ADR that decides the band layout — a third band changes
  `crates/nvs-diagnostics/src/lib.rs`'s legend and every tool grouping by band.
  [until: reviewed 2026-09-06]
