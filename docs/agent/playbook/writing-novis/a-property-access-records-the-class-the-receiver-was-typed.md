- **A property access records the class the receiver was typed as, not the class that declared the
  property.** `nvs_types::expr::members::check_property_member` writes `ExprInfo::Property { class:
  qname }` from the receiver — right for `InstKind::FieldGet`, wrong for any per-property fact
  `nvs-ir` looks up by that label: an inherited `lateinit` read through a subclass answers `false`
  against the parent's own-only record and the guard is silently not emitted. Flatten such a table
  along the class graph where it is recorded; the failure is quiet, because a method that never
  touches `$this` runs fine on a null receiver. [until: gone crates/nvs-types/src/expr/members.rs:check_property_member]
