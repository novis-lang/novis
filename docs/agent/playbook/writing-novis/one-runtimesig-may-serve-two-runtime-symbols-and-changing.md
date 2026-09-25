- **One `RuntimeSig` may serve two runtime symbols, and changing one symbol's declaration then
  miscompiles the other.** `nvs_array_unset` was emitted through `RuntimeSig::ArrayAppend`, so
  growing `nvs_array_append` would have given `nvs_array_unset` the wrong arity. Before editing an
  `extern "C"` in `nvs-runtime`, grep `crates/nvs-codegen/src/emit.rs` for its `RuntimeSig::`
  variant and give any second `runtime_ref` its own `Signatures` entry. [until: gone crates/nvs-codegen/src/emit.rs:RuntimeSig::ArrayAppend]
