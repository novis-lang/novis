- **A `Class::CONST` whose class does not exist passes checking and panics `nvs-ir`.** `echo
  \Core\Http\Method::Get;` type-checks clean and dies in `crates/nvs-ir/src/lower/expr.rs` with "a
  `Class::CONST` … with no value recorded in the typed-expression table", because `nvs_types::check`
  reports no `E0405` for the class half of that spelling. A one-line probe that compiles is not
  evidence that the name it writes resolves to anything — the same hole lets a payload roster naming
  an undeclared enum admit a case of the wrong enum. [until: reviewed 2026-09-06]
