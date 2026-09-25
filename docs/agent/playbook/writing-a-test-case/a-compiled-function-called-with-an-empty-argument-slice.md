- **A compiled function called with an empty argument slice faults.** `nvs_runtime::call(f, &mut
  ctx, &[])` on a method is an access violation (`0xc0000005`, a bare `STATUS_ACCESS_VIOLATION`
  naming no test), because the callee reads its argument slot regardless. Give the method one
  parameter it ignores and pass `Value::int(0)`, as `nvs-codegen`'s `stack_limit.rs` fixtures do; it
  bites only through `unit.function("Class::member")`, never `run_with`/`output_of`.
  [until: gone crates/nvs-codegen/tests/common/mod.rs:output_of]
