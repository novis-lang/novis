- **A rule added to `nvs_syntax::check_declarations` reaches far less of the corpus than a grep
  suggests.** Only `nvs-cli` and `nvs_hir::requires` call that walk, so every `nvs-types` fixture,
  parser test and `nvs-codegen` fixture goes straight past it — a `<?nvs` snippet in a Rust string
  is not automatically subject to everything the compiler enforces. Grep for the *callers* before
  budgeting a corpus rewrite. [until: reviewed 2026-09-06]
