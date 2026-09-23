- **A `-p nvs-stdlib` test can hand a `Core` member a real `callable`.** `nvs_runtime::call_closure`
  reads only `CLOSURE_ARITY_SLOT` and the `CLOSURE_INVOKE` address, so a `ClassTable::define` +
  `set_methods` pair with a plain `unsafe extern "C" fn` is a whole closure;
  `crates/nvs-stdlib/tests/allocation_policy.rs`'s `closure_of` is the shape, and leaks the table: a
  descriptor's address is its identity. The callee owes the exit sweep or valgrind catches it.
  [until: reviewed 2026-09-06]
