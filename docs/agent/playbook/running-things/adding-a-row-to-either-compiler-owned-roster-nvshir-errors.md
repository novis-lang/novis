- **Adding a row to either compiler-owned roster — `nvs_hir::errors::TREE` or
  `nvs_hir::interfaces::RESERVED` — fails a test in `nvs-ir`, and the message names neither the roster
  nor the name you added.** Both are restated as one hard-coded, alphabetically sorted label list in
  `lower/tests.rs`'s `a_file_with_no_class_still_carries_every_compiler_declared_class`, so the failure
  is a bare `left: [...]`/`right: [...]` diff in `-p nvs-ir --lib`. A new § 10 class owes the `TREE`
  row, `nvs_runtime::ThrownClass`'s variant, its `name()` arm, its `ALL` entry, that assertion, and the
  spec's own tree drawing; a new global interface owes the `RESERVED` row and that same assertion.
  [until: gone crates/nvs-ir/src/lower/tests.rs:a_file_with_no_class_still_carries_every_compiler_declared_class]
