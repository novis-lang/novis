# Handoff

## State

**M4 — language completeness.** `do`/`while` lowers (`mwl_ir::lower::Lowering::lower_do_while`,
`crates/mwl-ir/src/lower/control.rs:250`): the body's first block *is* the loop header, so the
phis sit above the body and the pre-loop edge falls straight into them, and `continue` targets a
condition block that every way an iteration can end flows into — [`lower_for`]'s step block in a
different role, merged the same way. The header therefore sees one back edge. A condition nothing
reaches (`do { return 7; } while (true);`) is sealed unreachable exactly as `lower_for` seals an
unreachable step block.

**A compound assignment to a static property lowers.** `Class::$p += 1`, `.=`, `++` and `--` all
work now: `reevaluable_target` (`crates/mwl-ir/src/lower/stmt.rs:1230`) answers `true` for
`ExprKind::StaticPropertyAccess`, which is the whole fix — `static_property_of` resolves the
`(declaring class, name, representation)` triple out of the typed-expression table, so the class
part is a name and the second read runs nothing. No staging, no new refcount edge: the store arm
already read the slot back and released it.

`verify.py` 6 of 6 green — conformance **616**, differential 173. `tools/leak-check.sh` clean over
a fixture that carries a `string` local through a `do`/`while` body and appends to a `static
string` in it.

**The worklist tool overstates what is open** — see the playbook's new Tooling bullet. Five of
`holes.py`'s six ranked items name features that already run; `--cases` is the half to trust.

## Next group

**A closure is called through the variable holding it.** The runtime half already exists
(`mwl_runtime::call_closure`, `crates/mwl-runtime/src/closure.rs:72`) and every `Core` member
taking a `callable` goes through it; what is missing is the lowering, so the four files below are
one slice's file set: `crates/mwl-ir/src/lower/expr.rs`, `crates/mwl-ir/src/ir.rs`,
`crates/mwl-codegen/src/emit.rs`, `crates/mwl-runtime/src/closure.rs`.

- [ ] **`$fn(args)` panics `mwl-ir` outright.** `crates/mwl-ir/src/lower/expr.rs:311` is the arm
      that refuses it — *"got `Call { callee: Variable(…) }`"*, which reads as a parser gap rather
      than the missing lowering it is. ADR 0031 § 1 says `callable` is the only closure type and a
      closure is an object of a synthesized class with one `invoke` method
      (`crates/mwl-ir/src/lower/closure.rs`), so the shape is a helper call at
      `crates/mwl-runtime/src/closure.rs:72` with the receiver and a slice of arguments — not a
      lowered `Call` with a resolved target. Add the `ir::Helper` row and its
      `crates/mwl-codegen/src/emit.rs` arm alongside.
- [ ] **`tests/conformance/lang/a-closure-is-called-through-the-variable-holding-it.mwlt`** — the
      named case stage 8 owes for it (`python tools/holes.py --cases`). Pin the arity trim
      `call_closure` performs (a closure declaring fewer parameters than it is handed), since
      `crates/mwl-runtime/src/closure.rs:113`'s `closure_arity` is the only thing that makes a
      `Core\Arr` callback and a hand-written one the same shape.
- [ ] **`mwl-ir`'s known-gaps 15 and 16 no longer describe the tree.**
      `crates/mwl-ir/src/lib.rs:355` says `<=>` "reaches `lower_expr`'s panic for every scalar
      operand" and `crates/mwl-ir/src/lib.rs:359` that "`$x++` and `--$x` do not lower"; both run.
      Rewrite the two entries around what is actually left (`**`/`**=` have no `ir::BinOp` row,
      and `f()->count += 1` is still refused), because this list is what `holes.py` and the next
      session read.

## Backlog

- `Core\Fault` cannot be constructed with arguments — `new Core\Fault("…")` panics
  `crates/mwl-ir/src/lower/expr.rs:3341` naming a `mwl_types` zero-arity gap. Use `RuntimeError`.
- 16 named `.mwlt` cases still owed; `python tools/holes.py --cases` is the list.
- `static::$prop` is `E0499` and an uninitialized non-nullable static is `E0409` — both owed a
  `--EXPECTF-ERROR--` case; `docs/adr/README.md` § *Decisions taken at project start* owns why.
- ADR 0007 § 2's `array<T> as array<U>` row still does not lower — the playbook cites it as the
  wall four separate case-writing traps run into.
- A spread argument does not lower (`crates/mwl-ir/src/lower/call.rs:85`), item 16.
- `E0499` remains the last code in the `E04xx` band; a band decision is owed before the next
  types diagnostic.
