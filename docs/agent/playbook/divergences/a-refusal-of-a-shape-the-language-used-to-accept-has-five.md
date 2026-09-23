- **A refusal of a shape the language used to accept has five homes, and two of them are tables.**
  The code in `crates/nvs-diagnostics/src/lib.rs`, the report site, a `tests/conformance/reject/`
  case, a row in `docs/reference/tools/30-php-differences.md`, and a `divergesFromPhp` note on the
  owning rule, which `python tools/rules.py --render` writes into `docs/divergences.md`. Then
  `python tools/reference.py` regenerates `docs/novis.md` and proves its examples, and any test that
  pinned the old rule is rewritten to the new one rather than deleted. [until: reviewed 2026-09-06]
