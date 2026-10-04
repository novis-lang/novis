- **A `-p nvs-stdlib` test can hand a `Core` member a real `callable`.** `nvs_runtime::call_callable`
  reads only `CALLABLE_ARITY_SLOT` and the `CALLABLE_INVOKE` address, so a `ClassTable::define` +
  `set_methods` pair with a plain `unsafe extern "C" fn` is a whole callable;
  `crates/nvs-stdlib/tests/allocation_policy.rs`'s `callable_of` is the shape, and leaks the table: a
  descriptor's address is its identity. The callee owes the exit sweep or valgrind catches it.
  [until: gone crates/nvs-stdlib/tests/allocation_policy.rs:nvs_runtime::call_callable]
