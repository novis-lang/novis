- **A `Core` option admits a qualifier in its *type*, never with a `Qual`.** `nvs_types::core_lib`'s
  `qual_of` answers `None` for an options-bag member, and `None` refuses a `tainted` or `secret`
  argument exactly as `Qual::Sink` does, so `array<string>` refuses a `secret string` however the
  option is marked. Declare the atom that carries the bits instead — `CoreTy::SecretTaintedStr` — and
  pair a type that admits `null` with `Const::NeverWritten` rather than `Const::Null`, which two
  `registry.rs` tests fail on by name. [until: gone crates/nvs-types/src/core_lib.rs:CoreTy::SecretTaintedStr]
