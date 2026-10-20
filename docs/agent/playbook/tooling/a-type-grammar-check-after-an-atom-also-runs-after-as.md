- **A type grammar check after an atom also runs after `as`, where the next `<` is a comparison.**
  `$n as int < 10` parses its target with the same `parse_type`, so a check that reads a `<` after
  `int` as a type argument list breaks ordinary comparisons across the conformance suite. Skip such a
  check while `in_as_target` is set, as `reject_builtin_type_args` in
  `crates/nvs-syntax/src/parser/ty.rs` does. `until: gone crates/nvs-syntax/src/parser/mod.rs:in_as_target`
