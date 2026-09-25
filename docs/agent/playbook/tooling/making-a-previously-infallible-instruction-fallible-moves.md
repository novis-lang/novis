- **Making a previously infallible instruction fallible moves two guards that name neither it nor
  the operator.** Each new `Inst::on_error` edge adds *two* machine-code `call`s (`nvs_raise_new`,
  then `nvs_trace_push` in the landing block), so `perf_guards.rs`'s
  `a_typed_arithmetic_loop_contains_no_call` needs a term per category, and `nvs-ir`'s
  `a_hook_body_reaching_its_own_property_touches_the_slot_directly` finds the function's name in a
  landing block's `propagate` label with no recursion. Both assert over rendered IR, so `grep` for
  the *edge* (`! bb`, `propagate`), not the operator. [until: gone crates/nvs-ir/src/lower/tests.rs:a_hook_body_reaching_its_own_property_touches_the_slot_directly]
