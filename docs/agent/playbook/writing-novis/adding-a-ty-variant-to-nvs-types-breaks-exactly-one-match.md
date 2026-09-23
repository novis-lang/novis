- **Adding a `Ty` variant to `nvs-types` breaks exactly one match, and it is not one you would
  guess.** Nearly every `match` over `Ty` in that crate has a `_` arm, so `Ty::ClassRef` compiled
  everywhere except `expr/operators.rs`'s `equality_domain` —
  `rule:expressions/disjoint-comparison-refused`'s domain partition, exhaustive on purpose so a new
  type cannot silently become comparable to everything. That is the one place a new variant owes a
  decision rather than an arm (a new `EqDomain` variant costs the enum, `equality_domain`, and
  `reject_unordered_operand`'s match); decide it in the ADR that adds the type, not at the compiler
  error. [until: gone crates/nvs-types/src/expr/operators.rs:fn equality_domain]
