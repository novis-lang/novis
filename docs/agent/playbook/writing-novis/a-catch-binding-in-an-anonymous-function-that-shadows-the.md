- **A `catch` binding in an anonymous function that shadows the enclosing frame's name is lowered as a capture,
  and the compiler panics.** The panic in `crates/nvs-ir/src/lower/expr.rs` reads "the anonymous function at …
  captures `$full`, which is not bound in the enclosing frame" and points at nothing near the
  `catch`. Rename the anonymous function's exception binding; a program hitting this panic is looking for a
  shadowed binding, not a capture it wrote.
  [until: gone crates/nvs-ir/src/lower/expr.rs:which is not bound]
