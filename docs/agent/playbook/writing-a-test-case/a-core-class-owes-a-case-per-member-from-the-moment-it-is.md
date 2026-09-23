- **A `Core` class owes a case per member from the moment it is registered, even before anything can
  produce an instance.** The gate is a text match, so registering a class whose only producer has
  not landed fails `cargo test -p nvs-stdlib`; the two are one slice. No server is needed:
  `tests/conformance/core/db-rows-answers-the-types-the-results-table-names.nvst` is the shape.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:const FLOOR: usize = 3]
