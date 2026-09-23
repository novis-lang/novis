- **`nvs_ir::Ty` is `#[non_exhaustive]`, so a `match` on it outside `nvs-ir` cannot be exhaustive.**
  The "a new representation is a decision, not a default" guard can only live in `nvs-ir` itself,
  and `nvs_ir::lower::param_tag_nibble` already *is* that guard. `nvs-codegen`'s `clif_ty` and
  `tag_of` end in a `_ =>` arm because the compiler requires one there, so do not try to hold a
  second copy in `ty.rs`. [until: gone crates/nvs-ir/src/ty.rs:non_exhaustive]
