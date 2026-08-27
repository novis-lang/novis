# Handoff

## State

**M4 — language completeness.** One group landed, closing the last of the previous
group's leftovers: **`if ($m != null) { echo $m["k"]; }` over a `?array<string>` runs**,
and so does every other `?T` a `!= null` test proves.

`mwl_types::locals`' `narrow` (`crates/mwl-types/src/locals.rs:315`) no longer restricts
the residue to a `Ty::Class` — it keeps whatever dropping `null` leaves. What made that
safe is the half below it: the narrowing is recorded on the **variable read's own span**
(`ExprInfo::NarrowedRead`, `crates/mwl-types/src/expr_table.rs:313`, from the
`ExprKind::Variable` arm at `crates/mwl-types/src/expr/mod.rs:143`) and discharged
**once**, where `mwl-ir` produces the value
(`Lowering::untag_narrowed`, `crates/mwl-ir/src/lower/expr.rs:1222`), instead of at each
consumer. That is the whole design: an array has several consumers — a subscript base, a
`foreach` subject, an array-write root, a call argument — and one forgotten `untag_array`
would have been a cranelift rejection rather than a panic. `untag_receiver` survives as a
no-op for the receiver it predates.

**The array-write root re-tags.** `write_back_array`'s `Variable` arm
(`crates/mwl-ir/src/lower/mod.rs:1818`) coerces back to `Ty::Tagged` when the local it is
writing into holds one, because the slot is one tagged slot wide however narrow the guard
is and the narrowing ends with the guard.

`E0482`'s nullable-array help now names the `!= null` test rather than the two workarounds,
and says that only a *binding* narrows — `Core\Arr::first($rows)["name"]` is still refused,
because a temporary has no test to narrow it. The playbook bullet that stated the old
restriction is rewritten in place rather than deleted, since the shape moved rather than
went away.

`verify.py` 6 of 6 green — conformance **579**, differential 162. `tools/leak-check.sh` is
green over two fixtures covering the new `Untag`/`Tag` edges (a narrowed array read, an
element write through one, a `foreach` over one, a narrowed property receiver).
`holes.py` is unchanged at **35 sites, 9 items** — this was never a panic.

## Next group

**All three are one gap in one helper**: an array literal placed against a `?array<T>`
*expectation* is typed `array<mixed>`, because the expectation's `null` is never stripped
before the element type is read off it. They share
`crates/mwl-types/src/expr/literals.rs`'s `check_array_literal` (line 596) and the three
call paths that hand it an expectation.

- [ ] **`?array<string> $m = ["k" => "v"];` is `E0401`** — *expected `null|array<string>`,
      found `array<mixed>`*. `check_array_literal`
      (`crates/mwl-types/src/expr/literals.rs:596`) reads the element type off `expected`
      and finds a union, so the literal falls back to `array<mixed>`; the fix is to strip
      `null` from the expectation first (`TypeInterner::without_null`, the same call
      `narrow` and `report_unsubscriptable` already make) and only then look for a
      `Ty::Array`. Reached from `crates/mwl-types/src/expr/mod.rs:161`.
- [ ] **The same literal in a `return` and in an argument** — `return ["k" => "v"];` from
      a `?array<string>` method is `E0403`
      (`crates/mwl-types/src/expr/assign.rs:301`), and `F::take(["k" => "v"])` against a
      `?array<string>` parameter is `E0401`
      (`crates/mwl-types/src/expr/args.rs:424`). Both are the same expectation reaching
      the same helper, so they fall out with slice 1 — but each needs its own `.mwlt` row,
      since nothing else asserts the two positions agree.
- [ ] **`$a["k"]["j"] ?? "d"` still throws at the inner level** (ADR 0007 § 7 row 11's
      exception cell). `Env::coalesce_guarded` marks only the `??`'s immediate left
      operand, so the inner read is unguarded; the `Index` arm
      (`crates/mwl-types/src/expr/mod.rs:303`) is where a marked read would have to push
      the mark down its own base. Decide there whether a guard covers the whole chain or
      only its last link, and say which in `Env::coalesce_guarded`'s doc.

## Backlog

- `holes.py` item 25's two sites are misattributed catch-alls — `decimal`, `never`,
  `iterable`, `self`/`static`/`parent`, a shape and an intersection as a *declared* type
  (`docs/implementation-plan.md` § Open now).
- ADR 0007 § 2's `array<T> as array<U>` does not lower, which is what blocks
  `Core\Csv::format`'s column refusal from ever being reached (playbook, *Writing a test
  case*).
- `instanceof`, a literal-typed comparison and `match (true)` still narrow nothing
  (`crates/mwl-types/src/lib.rs` known gaps).
- 20 of the 32 named `.mwlt` cases `tools/loop.py --list` owes are still to write.
- `docs/spec/02-php-migration.md` is 31% classified (`tools/check-migration.py`).
