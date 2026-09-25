- **A `Diagnostic`'s help line *is* a note, so "this refusal carries no note" is an assertion that can
  never hold.** `Diagnostic::with_help` pushes `help: …` onto the same `notes` vector `with_note` fills,
  so `d.notes.is_empty()` is false for any diagnostic that offers a fix at all, and a test asserting a
  *second* note is absent fails on the first one. Assert on a note's content — `n.contains("PHP 8.5")` —
  rather than on the vector's length. [until: gone crates/nvs-diagnostics/src/diagnostic.rs:self.notes.push(format!("help: {}"]
