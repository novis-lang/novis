- **A qualifier written on an element of an array literal is gone before any call-site rule looks at
  it.** `check_array_literal` (`crates/nvs-types/src/expr/literals.rs`) joins nothing — with no
  expectation a literal infers `array<mixed>` — so `Core\Json::encode(["token" => $secret])`
  compiles while `Core\Json::encode($secret)` and a declared `array<secret string>` are both
  refused; that is `rule:security/secret-qualifier`'s unmodelled container axis, not a hole in the
  sink. Probe the container spelling with a scratch `.nvs` before writing the case that claims it,
  or the case pins a refusal that never fires. [until: gone crates/nvs-types/src/expr/literals.rs:check_array_literal]
