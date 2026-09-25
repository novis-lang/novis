- **`nvs-ir` cannot name `nvs_hir::QName`.** `nvs-hir` is a *dev*-dependency there on purpose, so a
  lowering helper that wants one in its signature does not compile even though `nvs_types::Ty::Enum`
  hands it a `&QName` to pattern-match. Destructure it at the call site and pass what the callee
  needs (an `&EnumInfo`, or the name rendered with `to_string`); adding the dependency to get one is
  the wrong direction. [until: gone crates/nvs-ir/Cargo.toml:nvs-hir.workspace]
