- **A `Core` row can name a class `registry::CLASSES` does not hold, and only
  `every_instance_type_names_a_registered_class` stands in the way.** `Throwable` lives in
  `nvs_hir::errors::TREE` and is seeded by `nvs_types::error_lib`, so
  `CoreTy::Instance("Throwable")` resolves like any class; that test reads `CLASSES` alone and its
  `EXCEPTION_TREE` list is the second roster. A rule stated over `registry.rs`'s rows is not the
  whole rule wherever another crate answers for a class.
  [until: gone crates/nvs-stdlib/src/registry.rs:EXCEPTION_TREE]
