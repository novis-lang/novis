- **A fact `nvs-ir` and `nvs-runtime` both need lives in one of them and is held to the other by a
  test in `nvs-codegen`.** Neither crate names the other, so a shared constant has no crate both can
  see; `nvs-codegen` sees both, which is why `FN_ARITY`/`CLOSURE_ARITY_SLOT` are a pair with a
  codegen test between them and why the closure parameter-tag nibbles
  (`nvs_ir::lower::param_tag_nibble` writing, `nvs_runtime::Tag` reading) are held by
  `param_tag_nibbles_are_the_runtime_tag_bytes` in `crates/nvs-codegen/src/ty.rs`. A `pub fn` in
  `nvs-ir` plus a `#[test]` in `nvs-codegen` is the shape, not a new dependency edge.
  [until: reviewed 2026-09-06]
