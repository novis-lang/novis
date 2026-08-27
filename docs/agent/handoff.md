# Handoff

## State

**M4 — language completeness.** `lower_expr`'s dispatch catch-all
(`crates/mwl-ir/src/lower/expr.rs:259`) has had its cheap half taken: the four name-shaped
expressions that reach it are refusals now, not lowerings. A bare name in value position is
**E0319**, a bare name called is **E0320**, and `self`/`static`/`parent` in value position is
**E0321** — all in `mwl_hir::members`, whose module doc owns why resolution and not
`mwl_types`. Every class-side position skips the value walk through the new `walk_class_side`,
and a name the parser already refused as a top-level `function`/`const` is suppressed at its
use site so one mistake stays one diagnostic.

`verify.py` 6 of 6 green — conformance **605**, differential **167**, 1632 unit tests.
`python tools/holes.py` reads **24 sites, 6 items**, unchanged: the catch-all is one site
whichever shapes reach it.

**The enumeration the item asked for**, in `ExprKind` order, is what still reaches a lowering
panic. Already refused before lowering: `YieldFrom` (E0448), a `yield` outside a generator
(E0445), a `yield` in value position (typed `void`, so an ordinary E0401), `Error`
(unreachable), and the four this session took.

- `crates/mwl-ir/src/lower/expr.rs:259` — `PreIncDec`/`PostIncDec` in *expression* position
  (`echo $a++;`), `Assign` in expression position (`int $b = ($a = 2);`), `Call` on a callable
  value (`$f()`), `StaticPropertyAccess` (`C::$n`), `ClassNameConst` (`C::class`), `Isset`,
  `Empty`, and `Require` in expression position.
- `crates/mwl-ir/src/lower/expr.rs:247` — `ClassConstAccess` on a user-declared class.
- `crates/mwl-ir/src/lower/stmt.rs:300` — `Print`, `Exit` in both arities, `SpawnScript`.

## Next group

**The three shapes that both write and answer**, all in the two files this session's
enumeration anchored: `crates/mwl-ir/src/lower/expr.rs` with
`crates/mwl-ir/src/lower/stmt.rs` beside it. They are one group because the statement form of
each already lowers — the missing half is uniformly "and hand back a value".

- [ ] **`$x++` / `--$x` in expression position** — `holes.py --item 7`, 5 sites, the largest
      single item left. The statement form is `crates/mwl-ir/src/lower/stmt.rs:283`, which
      lowers the write and discards; the expression form is ADR 0007 § 4's `± 1` answering the
      target's own type, pre-form the new value and post-form the old.
      `crates/mwl-ir/src/lower/expr.rs:259` is where it lands.
- [ ] **An assignment in expression position** — `int $b = ($a = 2);`. Falls out of the slice
      above: `crates/mwl-ir/src/lower/stmt.rs:227` already lowers the write, and the value is
      the one it bound. `mwl_types` already types it (`crates/mwl-types/src/expr/mod.rs:213`).
- [ ] **`isset(...)` and `empty(...)`** — ADR 0028 § 3 fixes `isset($x)` as `$x != null` for
      every binding, which is `Lowering::lower_null_identity` per operand ANDed together;
      `empty($x)` is ADR 0035 § 2's truthy table negated, which is `Lowering::lower_not` over
      the same `truthy_convert` the condition slice uses. Both are typed `bool` already
      (`crates/mwl-types/src/expr/mod.rs:448`).

## Backlog

- `C::class` (`ClassNameConst`) is typed `mixed` at `crates/mwl-types/src/expr/mod.rs:291` and
  should be `string`; the lowering needs an `ExprInfo` carrying the rendered name, because
  `mwl-ir` cannot name `QName` (playbook). Owner: `mwl_types::expr_table`.
- `C::$n` (`StaticPropertyAccess`) resolves and type-checks but does not lower —
  `crates/mwl-types/src/expr/mod.rs:263` has the checker half already.
- `Class::CONST` on a user-declared class — `crates/mwl-ir/src/lower/expr.rs:247`; needs the
  `mwl_types` half (a constant's value is unmodeled there) before the lowering.
- `print`, `exit` and `spawn script` panic at `crates/mwl-ir/src/lower/stmt.rs:300`. Check ADR
  0049 § 1 for `exit`'s status before writing it — `die` is already a rejected synonym.
- **The `E04xx` band has two numbers left** (E0498, E0499). The next types-phase refusal after
  those needs a band-widening decision; the legend is `crates/mwl-diagnostics/src/lib.rs`'s own
  table.
- `mixed $p = require "f.mwl";` panics — `require` in expression position, ADR 0021.
