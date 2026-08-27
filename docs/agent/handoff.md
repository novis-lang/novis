# Handoff

## State

**M4 — language completeness.** Two slices landed, and between them they close the last of
the previous group: every `ExprKind::Index` shape that used to reach `mwl-ir` and panic is
now a diagnostic named where it is written.

`$a[]` outside a plain `=`'s target chain is **E0481** (`crates/mwl-diagnostics/src/lib.rs`).
The legal spans are marked by `mark_write_target_levels` walking the target chain *before*
the target is checked, because the arm that reports sits inside that check
(`crates/mwl-types/src/expr/assign.rs:399`). `unset($a[])` and `$a[] .= "x"` come along for
free — both route through `check_expr`'s `Index` arm — and the second of those is a
**deliberate divergence**: PHP appends there, the absent element reading as `""`. That is
ADR 0007 § 7 **row 10** now, and the playbook bullet owns how it was found.

A subscript whose base declares no element type is **E0482**: a `mixed`, a scalar, a
`string` (ADR 0009 § 2 indexes one through `Core\Str`, deliberately not through a
subscript), or a `?array<T>` that no test narrowed. Both `lower_index` panics and the
assignment arm's matching one are invariant checks no source file reaches now, so `mwl-ir`
gap 23 is gone and gap 6's opening clause is rewritten. Two suppressions keep it honest:
a base that already reported, and a write-target level whose chain root
`check_write_target` refuses by name — `refused_as_a_write_target`, in
`crates/mwl-types/src/expr/mod.rs`.

`verify.py` 6 of 6 green — conformance **576**, differential 162. No valgrind leg: nothing
in the lowering moved, only refusals above it. `holes.py` is at **35 sites, 9 items**.

## Next group

**All three share `crates/mwl-ir/src/lower/expr.rs`'s `lower_index` (line 3661),
`crates/mwl-ir/src/lower/mod.rs`'s `write_back_array` (line 1810) and
`crates/mwl-types/src/locals.rs`'s `narrow` (line 303).** The first is a live crash found on
the way to E0482 and should lead; the second is what makes E0482's nullable-array arm
unreachable.

- [ ] **Reading an absent key aborts the process** — `array<string> $a = []; echo $a["9"];`
      dies at `crates/mwl-runtime/src/string.rs:511` with *"null pointer dereference …
      non-unwinding panic"*, exit 0 and nothing on stderr. `InstKind::ArrayGet`
      (`crates/mwl-codegen/src/emit.rs:722`, `crates/mwl-runtime/src/array.rs:1258`) answers
      an absent key with a null-shaped value and every consumer then reads it as its
      declared type. PHP warns and yields `null`; MWL has no `null` to put in an
      `array<string>`, so under ADR 0007 § 2 the answer is a **throw**, the way ADR 0007
      § 5's own runtime normalization already throws. Note `write_back_array`'s
      `Helper::ArrayRowForWrite` is the *write* side's answer to the same question and is
      the shape to copy.
- [ ] **`?array<T>` does not narrow out of `null`** — `crates/mwl-types/src/locals.rs:303`
      narrows only a `null`-and-one-class union, on purpose: its doc says a wider residue
      would type-check and then panic in `mwl-ir`. An array is the one other case that
      *can* be untagged, and the move is the one `lower_reassignment`'s property arm
      already makes (`untag_receiver`, the fix in `write_back_array` two sessions ago).
      Landing it makes `if ($rows != null) { $rows["0"] }` run instead of taking E0482.
- [ ] **Re-word E0482's nullable-array help and retire the playbook's `?array<T>` trap**
      once the slice above lands — `crates/mwl-types/src/expr/mod.rs`'s
      `report_unsubscriptable`, and the *"A `?array<T>` cannot be indexed even after a
      `!= null` guard"* bullet in `docs/agent/playbook.md`.

## Backlog

- `mwl-ir` gap 6's remaining stale clause: a **static** property has no assignment target,
  so `C::$p = v` still panics (`crates/mwl-ir/src/lib.rs`).
- `lower_decl_type`/`lower_checked_ty`'s catch-alls (holes.py item 25's 2 sites) still cover
  `decimal`, `never`, `iterable`, `self`/`static`/`parent`, a shape and an intersection as a
  *declared* type — no item names that hole yet (`docs/agent/loop-goal.md`).
- `array<T> as array<U>` does not lower (`crates/mwl-ir/src/lower/expr.rs:877`), which is
  what blocks four separate `Core` depth cases — the playbook names them.
- The two unattributed `mwl-codegen/src/ty.rs` refusal sites (116, 121) belong to no item
  (`python tools/holes.py`).
- ADR 0007 § 7 is a ten-row table now; M11's `.phpt` pass-rate note in that section still
  says the count out loud, so a new row means editing the sentence under it.
