- **A `///` that links a test by a `tests::` path passes every session's gate and fails the goal's
  last one.** `mod tests` is `#[cfg(test)]`, so `cargo doc` resolves no `tests` item and
  `-D warnings` turns the link into `unresolved link` — and `bun nv verify` compiles the
  docs only under `--doc`, the gate a DONE claim needs and no ordinary session runs, so they
  accumulate across crates unseen. Name a test in backticks and never in brackets, and when `--doc`
  is red on one, `grep -rn 'tests::' --include=*.rs crates/` finds every sibling in one call.
  [until: reviewed 2026-09-16]
