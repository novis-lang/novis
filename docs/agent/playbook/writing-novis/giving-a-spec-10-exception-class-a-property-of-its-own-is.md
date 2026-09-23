- **Giving a spec § 10 exception class a property of its own is six edits across four crates, and
  the two that are not `match` arms bite.** `nvs_hir::errors::OWN_PROPERTIES`, its `*_SLOT` and
  `error_lib::own_properties`'s type arm are loud; the quiet ones are
  `nvs_ir::lower::exception::synthesized_exception_constructors`, a hand-kept list nothing ties to
  the row, and the functions list of
  `a_file_with_no_class_still_carries_every_compiler_declared_class`. Land all in one change;
  `INSTA_UPDATE=always python tools/verify.py -p nvs-ir` rewrites the snapshots that go red.
  [until: reviewed 2026-09-06]
