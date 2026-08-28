# Handoff

## State

**M4's Stage 6 is the frontier, and ADR 0007 § 6's narrowing list is half
landed.** `nvs_types::locals`' `narrow` now tries three residue functions —
`null_residue`, `instanceof_residue`, `literal_residue` — and that module's own
narrowing section is the one home for what each proves and on which edge. The
plan's *Open now* paragraph carries why each is shaped the way it is.

- **Two spellings of § 6's four remain**: `match (true)` narrowing per arm, and
  the enum-case form of the literal comparison (`$m == Mode::Read`), which needs
  a case's *type* where `ExprInfo::EnumCase` carries only its backing value.
- **An `instanceof` against an interface narrows nothing**, deliberately —
  `instanceof_residue`'s doc comment owns why, and
  `an_instanceof_against_an_interface_narrows_nothing` is the test that keeps it
  from being loosened by accident.
- **`nvs-ir` still panics on an instance method call through a `mixed`
  receiver** (`crates/nvs-ir/src/lower/expr.rs:2389`), which is why the new
  `.nvst` tests its unnarrowed edge with a `?Dog` rather than a `mixed`.
- M4's acceptance still names *Verification* sections for ADRs 0023, 0028 and
  0069; 0014 and 0046 have theirs.

## Next group

**ADR 0007 § 6's last narrowing spelling, and the enum case with it.** One file
set: `crates/nvs-types/src/locals.rs` (`narrow`, `locals.rs:344`;
`literal_residue`, `locals.rs:476`; `literal_test`, `locals.rs:513`; the
`Switch` arm, `locals.rs:933`) and `crates/nvs-types/src/expr_table.rs`
(`ExprInfo::EnumCase`, `expr_table.rs:491`).

- [ ] **`match (true)` narrows per arm** — ADR 0007 § 6. Each arm's label is a
      condition, so the arm body is checked under exactly what `narrow` would
      install for it; the `Match` expression arm and the `Switch` statement one
      (`locals.rs:933`) are the two sites, and a `switch` case falling through
      is the existing known gap the module doc already names.
- [ ] **An enum-case comparison narrows** — ADR 0047 § 4's guard row, the half
      `literal_residue` deliberately left. It needs the case's *type*
      (`Ty::EnumCase`) where `ExprInfo::EnumCase` (`expr_table.rs:491`) records
      only its backing value, so the slice is either a second field on that
      variant or the enum name resolved beside it.
- [ ] **A narrowed subject that reaches `nvs-ir` as a method call** — the panic
      at `crates/nvs-ir/src/lower/expr.rs:2389`: a `mixed` receiver's instance
      call has no resolved target, so `$m->speak()` on an unnarrowed `mixed`
      fails the whole compilation naming a compiler gap rather than the program.

## Backlog

- A `require` whose path is not a string literal runs nothing, silently, in both
  forms — `nvs_hir::requires`' own known gap.
- `nvs_stdlib::debug` gap 1: an ADR 0036 shape field and an `array<T>` element
  carry no `secret` bit (ADR 0033's unmodelled container axis).
- A user-declared class constant's declared type is unmodelled —
  `signatures.rs`' own known gap, which is what forced `E0731`.
- M4's acceptance owes *Verification* sections for ADRs 0023, 0028 and 0069.
