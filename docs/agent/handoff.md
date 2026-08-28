# Handoff

## State

**M4. Item 25 is one row from closed.** ADR 0007 § 3's `void`/`never` are return-only as a *rule*
now — `E0742` refuses either in a parameter, in the one pass that reads every declared signature
(`crates/nvs-types/src/signatures.rs:1013`, `reject_void_or_never_params`), so an interface and an
abstract method are covered as well as a body. `iterable` and an intersection got the other kind of
answer, a representation: `crates/nvs-ir/src/lower/mod.rs`'s `erase_checked_ty` maps `Iterable` to
`Ty::Tagged` and `Intersection` through the same member fold the union arm uses, now shared as
`shared_erasure`. `crates/nvs-ir/src/lib.rs`'s known-gap entry owns why neither arm makes its type
usable; do not restate it.

`KNOWN_ICE` in `crates/nvs-ir/tests/type_atoms.rs:110` holds **one** row: `("never",
Position::Return)`. Emptying it closes item 25.

**The ledger's acceptance failure after session 0005 does not reproduce.** It named
`every_spellable_expression_reaches_a_diagnostic_or_an_ir` as "did not run"; `cargo test -p nvs-ir`
runs it and it is green, both before and after this session's edits. Nothing was changed for it. If
the driver reports it again with the tree green, the bug is in how `loop.py`'s `cargo-named` check
collects that binary's output, not in the test.

## Next group

**Closing item 25, then the two diagnostics beside it.** One file set:
`crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-ir/tests/type_atoms.rs`, and `nvs-types` for the
checker halves.

- [ ] **`never` in a return reaches a representation or a diagnostic**, emptying `KNOWN_ICE` and
      closing item 25. The arm is `crates/nvs-ir/src/lower/mod.rs:2808` (`erase_checked_ty`); the
      row to delete is `crates/nvs-ir/tests/type_atoms.rs:110`. A function declaring `never` cannot
      return, so the open question is whether it has a value to represent at all — `Ty::Void` is
      the candidate, and the `check.rs:635` "leaves the frame" rule already treats the two alike.
- [ ] **ADR 0066 § 3's two refusals are diagnostics** (item 27). `$i as ?string` (cannot fail) and
      `$arr as ?int` (does not exist) reach `crates/nvs-ir/src/lower/convert.rs:402`
      (`convert_or_null`) and panic naming the ADR; the fix is in `nvs_types`, beside the `as` grid.
- [ ] **A nullsafe assignment target is a diagnostic** (item 29). `$a?->b = v` panics at
      `crates/nvs-ir/src/lower/mod.rs:1702`; § *Standing decisions* pre-authorizes the refusal and
      the array-element-through-a-hooked-property half with it.

## Backlog

- `nvs_types::expr::assign` has no arm for `iterable` or an intersection, so neither is inhabitable
  even now that both have a representation — `docs/agent/loop-goal.md` item 25's own bullets.
- A **closure** parameter is lowered at `crates/nvs-types/src/expr/calls.rs:1057`, not through
  `signatures.rs`, so `fn (void $p) => …` is not covered by `E0742`. Same for a property and a
  local declaring `void`/`never`.
- Item 26 — `bool as int`/`bool as string`, `array<T> as array<U>`'s element walk, and a
  `Ty::Tagged` operand converted to `bytes` (`nvs-ir` gap 20).
- Item 16 — ADR 0027's `Class::method(...)`, the single `SHAPE_ICE` row in `type_atoms.rs`.
- Stage `1 floor`'s fixture list is the first work behind the ten green guard tests of Stages
  `0a inout` and `0 operators` — `docs/agent/loop-goal.toml`.
