- **A valgrind stack whose allocation site is `NvsArray::make_unique` under `nvs_array_set` names an
  array *literal*, not the array write path.** The empty array is a per-thread singleton, so
  `nvs_codegen`'s `emit_array_new` chain always separates on its first `nvs_array_set`, which makes
  `make_unique` the allocation site of every non-empty literal in the program. Read such a report as
  "an array literal was never released", not as a dropped copy-on-write clone.
  [until: gone crates/nvs-runtime/src/array.rs:fn make_unique]
