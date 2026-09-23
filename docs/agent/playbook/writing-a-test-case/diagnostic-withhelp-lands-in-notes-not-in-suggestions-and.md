- **`Diagnostic::with_help` lands in `notes`, not in `suggestions`, and the struct has a
  `suggestions` field to lead you the wrong way.** `with_help` pushes `format!("help: {text}")` onto
  `notes` (`crates/nvs-diagnostics/src/diagnostic.rs`), while `suggestions` is only ever a
  machine-applicable `Suggestion { message, replacement, span }` and is empty for every
  configuration diagnostic in the tree, so a refusal test reading `refused.suggestions` compiles and
  then fails on an empty vector. Assert a help sentence as `refused.notes.iter().any(...)`.
  [until: reviewed 2026-09-06]
