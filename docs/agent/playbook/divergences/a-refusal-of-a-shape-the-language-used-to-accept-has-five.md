- **A refusal of a shape the language used to accept has five homes, and two of them are tables.**
  The code in `crates/nvs-diagnostics/src/lib.rs`, the report site, a `tests/conformance/reject/`
  case, a row in `docs/reference/tools/30-php-differences.md`, and a `divergesFromPhp` note on the
  owning rule, which `bun nv rules --render` writes into `docs/divergences.md`. Then
  `bun nv reference` regenerates `docs/novis.md` and proves its examples, and any test that pinned
  the old rule is rewritten to the new one rather than deleted.
  [until: gone tools/nv/cmd/rules.ts:divergences.md]
