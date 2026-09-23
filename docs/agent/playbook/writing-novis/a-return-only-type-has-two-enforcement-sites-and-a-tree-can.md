- **A "return-only" type has two enforcement sites, and a tree can have neither while looking like
  it has one.** `rule:types/grammar`'s `void`/`never` were return-only in prose alone: `never $p`
  panicked `nvs-ir`'s `erase_checked_ty`, while `void $p` type-checked, lowered, and died in codegen
  with `internal error: reading a value of representation 'void'` — a third outcome
  `crates/nvs-ir/tests/type_atoms.rs` cannot see, because it stops at `lower_program`. Two minutes
  of `nvs run` on a two-line scratch file tells a shape that panics, one that is refused, and one
  that reaches codegen apart. [until: gone crates/nvs-ir/tests/type_atoms.rs:lower_program]
