- **A closure captures by value, so a `.nvst` case cannot count anything by incrementing a captured
  variable.** `var $seen = 0; var $f = fn (): void => { $seen = $seen + 1; };` compiles, runs and
  leaves `$seen` at `0` however often `$f()` is called, and the case fails on a line that looks like
  a bug in the member under test. `echo` from inside the closure, or count on the outside.
  [until: gone crates/nvs-ir/src/lower/tests.rs:a_closure_lowers_to_a_captured_environment_object_and_an_invoke_method]
