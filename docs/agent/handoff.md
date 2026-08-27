# Handoff

## State

**M4 — language completeness.** One slice landed, and like the previous one it closes a live
wrong answer rather than a worklist item: **`$a["k"] ?? "d"` yields the default now** instead
of throwing.

`mwl_types` marks the `??`'s immediate left operand in `Env::coalesce_guarded`
(`crates/mwl-types/src/lib.rs`) before it checks it; the `ExprKind::Index` arm answers
`?elem_ty` for a marked read and records `ExprInfo::Index { guarded: true }`. That `null` in
the operand's static type is the whole mechanism — it is what stops `lower_coalesce`
(`crates/mwl-ir/src/lower/expr.rs:1297`) short-circuiting a `??` whose left operand looked
statically non-nullable. Below, `mwl_ir::ir::InstKind::ArrayGet` carries an
`mwl_ir::ir::AbsentKey`: `Throws` is every read written outside a guard (fallible, element
representation, `mwl_array_required_get`), `Null` is the guarded one (infallible, `Ty::Tagged`,
`mwl_array_optional_get` — new, in `crates/mwl-runtime/src/helpers.rs`). ADR 0007 § 7 row 11
carries the exception in its own cell; `AbsentKey`'s and `ExprInfo::Index::guarded`'s doc
comments own the two halves of the mechanism.

**Only the immediate operand is guarded.** `$a["k"]["j"] ?? "d"` still throws at the inner
level, because guarding a whole chain needs the base of a guarded read to be a `?array<T>`
that resolves an element type — which is the next slice. `Env::coalesce_guarded`'s doc says so.

`verify.py` 6 of 6 green — conformance **578**, differential 162. `tools/leak-check.sh` is
green over two fixtures covering the new edges (a guarded read off a call temporary, and a
rendered `uint` key under a guard). `holes.py` is unchanged at **35 sites, 9 items** — this
was never a panic. One existing case, `reading-an-absent-array-key-throws.mwlt`, had frozen
the old wrong answer into its `--EXPECT--` and is corrected.

## Next group

**All three share `crates/mwl-types/src/locals.rs`'s `narrow` (line 303, the residue test at
line 315) and `crates/mwl-types/src/expr/mod.rs`'s `Index`/`Variable` arms (lines 295, 143).**
The first is the one the other two wait on, and it is **not** the one-line residue widening it
reads as — the design work is done below, so do not re-derive it.

- [ ] **`?array<T>` does not narrow out of `null`.** `narrow`
      (`crates/mwl-types/src/locals.rs:315`) drops `null` only when the residue is a
      `Ty::Class`, so `if ($m != null) { echo $m["k"]; }` over a `?array<string>` is `E0482`.
      Widening the residue test is one line; what it costs is that `mwl-ir` lowers a `?T`
      local's slot as `Ty::Tagged` whatever the checker proves, and today the *only* consumer
      that reads back out of one is a receiver, through `Lowering::untag_receiver`
      (`crates/mwl-ir/src/lower/expr.rs:1212`). An array has several consumers, not one:
      `lower_index`'s base (`:3667`), the array-write root's write-back
      (`crates/mwl-ir/src/lower/stmt.rs:715`), a `foreach` subject, and a call argument.
      **Two shapes, and the second is the recommendation.** (a) An `untag_array` mirroring
      `untag_receiver` at each of those consumers — contained, blessed by precedent, but one
      forgotten site is a cranelift rejection rather than a panic. (b) Record the narrowing on
      the *variable read's own span* (`ExprInfo::NarrowedRead { to }`, from the
      `ExprKind::Variable` arm at `crates/mwl-types/src/expr/mod.rs:143`, where
      `LocalScope::declared_ty` already answers the narrowed type) and untag once in
      `lower_expr`'s `Variable` arm, so every consumer sees the narrow representation with no
      site to forget. (b) subsumes `untag_receiver`, and `narrow`'s own doc comment
      (`locals.rs:290-302`) is the paragraph to rewrite either way — it currently states the
      restriction as gated on `mwl-ir` gap 1's tagged arithmetic.
- [ ] **`?array<T> $m = ["k" => "v"];` is `E0401`** — *expected `null|array<string>`, found
      `array<mixed>`*. The expectation is not pushed through the `null` union to the literal,
      so a `?array<T>` binding can be initialised only from `null` or a call, which is what
      makes the slice above hard to even write a case for. `check_array_literal`'s `expected`
      (`crates/mwl-types/src/expr/mod.rs:151`) is the site.
- [ ] **Re-word `E0482`'s nullable-array help and retire the playbook's `?array<T>` trap** —
      `report_unsubscriptable`'s help text at `crates/mwl-types/src/expr/mod.rs:515` names a
      workaround that the first slice removes, and the playbook bullet under *Writing MWL
      itself* is the one to edit (not delete: the three spellings it lists stay useful).

## Backlog

- `$a["k"]["j"] ?? "d"` throws at the inner level where PHP yields the default — blocked on the
  `?array<T>` narrowing above; `mwl_types::Env::coalesce_guarded`'s doc owns why.
- `isset($a["k"])` should take the same guard once `isset` lowers — `docs/adr/0007` § 7.
- `mwl-ir` gap 1's tagged arithmetic: a narrowed `?int` would type-check `$n + 1` and fail in
  `emit_binop` — `crates/mwl-types/src/locals.rs:299` states the trade.
- `array<T> as array<U>` still does not lower (`crates/mwl-ir/src/lower/expr.rs:877`), which is
  what keeps four `gaps.py --errors` sites unreachable — playbook, *Writing a test case*.
- `Core\Json::decodeAs<T>`'s wider codec-reachable set and its two default-bearing rows —
  `mwl_stdlib::json`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
