# Handoff

## State

**Stage 0 items 1–7 and 9 are done; item 8 is untouched and is all that stands between the loop and
`Core` breadth.** ADR 0028 § 1's implicit string conversion now runs end to end: all four of its sites —
`.`, an interpolated piece, `echo`/`print` and `as string` — desugar to one virtual `toString()` call, so
an override wins and a `toString` body may throw like any other. The mechanism is the one `mwl-ir`'s gap
12 named: `mwl_types::expr::operators::require_stringable` is the single check all four sites go through,
so it records the resolved target in a new `ExprTypeTable::to_string_call` side map (a side map, not an
`ExprInfo` variant, because the operand usually already records an entry under that same span — the
`foreach_drive` precedent), and `mwl_ir::lower::Lowering::lower_to_string_call` reads it back.
`mwl_hir::implements_interface` is reflexive now, which is what lets a value typed at `Stringable` itself
stringify and one typed `Comparable` order through `<`; that function's doc says why the zero-step walk is
the answer all five of its callers wanted.

Cost: one `ResolvedCall` per object-typed implicit-conversion site in the compiled unit, freed with the
rest of the check run. At run time, one `ClassDescOf` + one virtual call where PHP does the same dispatch.

`python tools/verify.py` is green (1371 tests), `mwl test tests/` is 432/0, and both new fixtures are
valgrind-clean on the WSL leg.

## Next group — item 8, ADR 0061's `autoload`, in three slices

**Shared file set:** `crates/mwl-syntax/src/parser/decl.rs`, `crates/mwl-syntax/src/parser/stmt.rs`,
`crates/mwl-syntax/src/ast.rs` and `crates/mwl-hir/src/requires.rs`. The rule is
[`loop-goal.md`](loop-goal.md) § *Stage 0* item 8; the grammar is
[`docs/spec/00-overview.md:78`](../spec/00-overview.md#L78) § 2, and the semantics are
[ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md).

- [ ] **8a — the two file-scope declaration forms parse.** `autoload 'Prefix' from 'a', 'b';` and
      `autoload discover 'glob';`, spec § 2's grammar block verbatim. New `Keyword::Autoload` in the
      lexer, a new AST node, and a `parse_autoload_decl` next to
      [decl.rs:214](../../crates/mwl-syntax/src/parser/decl.rs#L214)'s `parse_use_decl` — the same
      file-scope-form-that-is-not-a-statement shape, dispatched from
      [stmt.rs:199](../../crates/mwl-syntax/src/parser/stmt.rs#L199). `loop-goal.toml` waits on
      `an_autoload_declaration_parses` in `mwl-syntax`.
- [ ] **8b — name-to-file resolution as a fixpoint** over the require-graph worklist
      [`resolve_program`](../../crates/mwl-hir/src/requires.rs#L99) already walks: an unresolved class
      name is mapped through each declared prefix to a candidate path, the file is parsed and folded into
      the same worklist, and the loop runs to a fixpoint. `loop-goal.toml` waits on
      `an_autoload_declaration_resolves_a_name_to_its_file` in `mwl-hir`.
- [ ] **8c — the `.mwlt` cases**, in `tests/conformance/lang/`. A class reached only through `autoload`,
      and a name no rule resolves. Check the second's diagnostic exists before writing the case; ADR 0061
      says what it should be, and E03xx's next free code is E0315.

## Backlog

- `mwl-ir` gap 12's whole remainder: a `Core`-owned class is exempt from `require_stringable`, so
  `echo $someCoreObject` panics. Saying which `Core` classes stringify is `mwl_stdlib::registry`'s answer.
- `mwl-ir` gap 21: a binding declared at the opaque `object` top has no representation arm, so
  `object $o = $obj;` panics — that gap's text says what a session landing it owes.
- ADR 0094's one uncovered shape: a promoted constructor parameter is no table's property, so nothing
  resolves it to check — `mwl-types`' `signatures` known gaps. `private(set)` is § 3's write half and is
  not modeled at all.
- ADR 0043 § 4's `by $field` delegation exempts a whole class from conformance — `mwl-types`'
  `conformance` module doc.
- No override-compatibility check exists, so ADR 0013 § 1's `compareTo(self)` variance rule is unenforced
  — `mwl-types`' `signatures`.
- `mwl-ir` gap 19: `$n + $f` and `$n < $f` type-check and still fail in codegen.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.

## Orientation gaps

`orient.py`'s `adrs` manifest printed ADR 0047's six sections, none of which this group needed, and not
ADR 0028 § 1, which it worked from throughout. `loop-goal.toml` is already updated for the next group —
`adrs` now names ADR 0061 §§ 1, 2, 4, 5, and `modules` adds `parser/stmt.rs`, `requires.rs` and
`resolve.rs`. Item 8 needs no `mwl-types` file at all, so trim that half of `modules` if it stays quiet.
