# Handoff

## State

**M4 — language completeness.** Item 7 is closed: `$x++` and `--$x` answer a value now, the
prefix form the new number and the postfix form the old, through the same
`Lowering::lower_read_modify_write` the statement form and `$x += 1;` already took
(`crates/mwl-ir/src/lower/stmt.rs:422`). Every target ADR 0007 § 4 leaves is non-refcounted,
so neither answer owes a retain.

**The enabling change is crate-wide and is what the next slices inherit.** Expression
lowering holds an `&mut Env` (`crates/mwl-ir/src/lower/expr.rs:44`), so a value-position
rebinding has a binding to re-point; every conditional operand rebinds into its own copy and
the edges meet at `merge_envs` — `&&`/`||`, ternary, `??`, `match`, and `?->` (whose guard
now carries a `pre_env`). And the loop-header phi scan is a full recursive walk of every
sub-expression and every statement clause, plus the loop's **own condition**
(`crates/mwl-ir/src/lower/control.rs:1956`). That last one is not
optional: without it `while ($i++ < 3)` spins forever on the pre-loop value.

`verify.py` 6 of 6 green — conformance **606**, differential **168**, 1632 unit tests.
`python tools/holes.py` reads **24 sites, 6 items**; item 7's five remaining are other items'
panics sharing `lower/stmt.rs`.

## Next group

**The two remaining value-position writers**, both in files this session already changed and
both unblocked by the `&mut Env` widening above: `crates/mwl-ir/src/lower/expr.rs` with
`crates/mwl-ir/src/lower/stmt.rs` beside it. Take them in this order — the third is the
cheapest of the group and shares only the dispatch.

- [ ] **An assignment in expression position** — `int $b = ($a = 2);`, and `$a = $b = 0;`.
      `ExprKind::Assign` still reaches `lower_expr`'s catch-all at
      `crates/mwl-ir/src/lower/expr.rs:278`; the statement form is
      `crates/mwl-ir/src/lower/stmt.rs:227`, and every compound form already
      goes through `lower_read_modify_write` (`stmt.rs:422`), which hands back both values.
      The answer is the value **written**, at the target's declared representation, and it is
      a second owner of whatever the binding now holds — so unlike an increment this one does
      owe a retain for a refcounted target. `collect_reassigned_in_children` already walks
      into an `Assign`'s target and value, so the loop-header phis need no further work.
- [ ] **`isset(...)` and `empty(...)`** — ADR 0028 § 3 fixes `isset($x)` as `$x != null`, and
      ADR 0035's truthy table answers `empty`. Both still reach the catch-all at
      `crates/mwl-ir/src/lower/expr.rs:278`; `Lowering::truthy_convert`
      (`expr.rs:1055`) is the whole of `empty`'s second half, and an absent array key is the
      one case that has to answer without throwing — see `InstKind::ArrayGet`'s `AbsentKey`,
      which `??` already uses for exactly that.
- [ ] **`Print` and `Exit` in expression position** — `crates/mwl-ir/src/lower/stmt.rs:300`
      still panics for both arities of `Exit`, and `Print` answers `1` in PHP. Cheap, and it
      shares only the dispatch with the two above.

## Backlog

- `$n + Adder::bump($n)` still reads the pre-call value — `pending_refs` is drained at the
  statement, and the doc there now says the scoping reason is gone and the sequence-point
  question is what is left (`crates/mwl-ir/src/lower/mod.rs:1135`).
- `mwl-codegen/src/ty.rs:116` and `:121` are `holes.py`'s two unattributed sites; no item names them.
- An enum case tagged into a `mixed` reads as its backing integer, so a `0`-backed case is falsy
  where ADR 0035 § 4 makes it truthy — `mwl_codegen::ty::tag_of`.
- `Core\Json::decodeAs<T>`'s wider codec-reachable set and its two default-bearing rows —
  `mwl_stdlib::json`'s own gaps.
- ADR 0088's qualifier classification — `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
