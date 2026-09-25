- **A fixture that needed a `mixed` value has probably reached for whatever inferred `mixed` that
  week, and closing a gap moves it.** `an_array_index_through_a_mixed_base_defers_to_the_tag`
  (`crates/nvs-types/src/expr_table.rs`) used `T::UNTYPED[0]` over an unannotated constant purely
  because that inferred `mixed`; giving constants a type turned it into an `int` subscript and
  failed the deferral it was written to assert. Give such a fixture the erasure it means — a `mixed`
  parameter — and when a slice widens what the checker knows about a shape, `grep` the test tree for
  that shape used as a *source*. [until: gone crates/nvs-types/src/expr_table.rs:an_array_index_through_a_mixed_base_defers_to_the_tag]
