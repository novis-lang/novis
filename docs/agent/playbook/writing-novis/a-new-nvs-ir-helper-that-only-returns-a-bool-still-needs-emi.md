- **A new `nvs_ir::Helper` that only returns a `bool` still needs `emit_fallible`, or codegen stops
  with `E0901: a status-returning instruction with no error edge`.** Every `nvs_helper!` entry point
  returns a status word, so a plain `emit` of a `HelperCall` with a result fails at codegen, not at
  build or lowering. Emit it the way `Helper::Identical` is emitted in
  `crates/nvs-ir/src/lower/convert.rs`. `until: gone crates/nvs-ir/src/lower/convert.rs:helper: Helper::PropertyPathValid`
