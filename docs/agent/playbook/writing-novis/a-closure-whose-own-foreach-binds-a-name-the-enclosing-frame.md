- **A closure whose own `foreach` binds a name the enclosing frame also bound panics `nvs-ir` rather
  than compiling.** `nvs_types` records the inner use as a *capture* because its scope resolved the
  outer `foreach` variable, and lowering then finds no slot for it —
  `crates/nvs-ir/src/lower/expr.rs:2667`, reproduced by nothing more than two `foreach`es over an
  `array<string>` with one variable name between them. Give the closure's loop variable a name of its
  own; the outer one is not what the closure meant either way.
  [until: test a_closure_binds_its_own_foreach_variable_over_an_enclosing_one]
