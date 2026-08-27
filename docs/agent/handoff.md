# Handoff

## State

**M4 — language completeness.** Two slices landed this session, both in the element-write
file set and both pinned by a new `tests/conformance/array/` case.

An element write through a **narrowed nullable receiver** now runs. It was never a panic, so
no worklist item named it: `write_back_array`'s property arm
(`crates/mwl-ir/src/lower/mod.rs:1822`) lowered the receiver and handed it straight to
`emit_field_set`, and a `?T` local is one tagged slot wide however narrow a condition proves
it — so cranelift rejected the function with *"invalid pointer width (got 128, expected
64)"*. One `untag_receiver` call, the same one `lower_reassignment`'s property arm already
made, is the whole fix; the playbook bullet owns how to recognise that message.

**An append at an intermediate level lowers** — `$g[][0] = 1`, PHP's "start a fresh row and
write into it". The flatten in `crates/mwl-ir/src/lower/stmt.rs:798` walks every `Index`
level rather than only the subscripted ones, `inner_keys` carries an `Option` per level, an
append level's row is an empty `InstKind::ArrayNew` (it has nothing to descend into) and the
climb stores it back with `InstKind::ArrayAppend`. All six spellings checked byte-for-byte
against PHP, including the append counter landing one past the highest integer key.

`verify.py` 6 of 6 green — conformance **574**, differential 162 — and both fixtures are
clean under `tools/leak-check.sh` (the new `ArrayNew`/`ArrayAppend` positions are a new
refcount edge). `holes.py` is unchanged at **9 items, 38 sites**: neither slice was a
counted refusal site.

## Next group

**All three share `crates/mwl-types/src/expr/mod.rs`'s `ExprKind::Index` arm (line 295),
`crates/mwl-types/src/expr/assign.rs`'s target walk (line 402) and
`crates/mwl-diagnostics/src/lib.rs`.** The first two are the same edit seen from two sides —
what an `Index` whose base or index is missing should say instead of panicking below.

- [ ] **`$a[]` in a *read* position is a diagnostic, not a panic** — `mwl-ir` gap 23,
      `crates/mwl-ir/src/lib.rs`. PHP refuses `echo $a[];` at compile time (*"Cannot use []
      for reading"*); MWL's front end accepts it and
      `crates/mwl-ir/src/lower/expr.rs:3669` panics. The legal spans are exactly the ones
      `check_assign` already walks down the target chain
      (`crates/mwl-types/src/expr/assign.rs:402`), so mark them there and report the new
      code from `crates/mwl-types/src/expr/mod.rs:295` for any other `index: None`. Next
      free is **E0481** (`crates/mwl-diagnostics/src/lib.rs:825` is E0480's row).
- [ ] **`mixed $m; $m["0"] = 1;`** — the third slice of the previous group, still untaken.
      `crates/mwl-ir/src/lower/stmt.rs:810` panics with *"an array-index assignment target
      … has no resolved element type"* because `check_expr`'s `Index` arm answers `mixed`
      for a non-array base (`crates/mwl-types/src/expr/mod.rs:303`, the `None =>` arm) and
      records no `ExprInfo::Index` at all. Decide there: either a `mixed` base is refused
      where it is written, or it lowers through the tagged representation.
- [ ] **`unset($a[]);` and `$a[] .= "x";`** — the two remaining append spellings nobody has
      run. `unset` goes through `crates/mwl-ir/src/lower/stmt.rs`'s
      `lower_unset_index`/`write_back_array` pair, compound assignment through
      `check_compound_assign`; PHP refuses `unset($a[])` at compile time, so this is likely
      the same E0481 and one more `.mwlt` case rather than new lowering.

## Backlog

- `holes.py` item 1 (ADR 0007 § 4's promotion table) is 12 of the 38 remaining sites — the
  largest single item left; `docs/implementation-plan.md` § *Open now* owns it.
- Two unattributed sites in `crates/mwl-codegen/src/ty.rs:116,121` that no item anchors.
- `lower_decl_type`/`lower_checked_ty`'s catch-alls still lack `decimal`, `never`,
  `iterable`, `self`/`static`/`parent`, a shape type and an intersection as a *declared*
  type — misattributed to item 25 by `holes.py`.
- 20 of the 32 named `.mwlt` cases `tools/loop.py --list` owes are still to write.
- `docs/spec/02-php-migration.md` is 31% classified (`tools/check-migration.py`).
