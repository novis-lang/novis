# Handoff

## State

**M4 — language completeness.** Item 16 is closed: a `name:` argument lands in its own
parameter's ABI slot while evaluating where it was written, a `...` argument's entries
are spread into the variadic tail, and both are pinned against PHP. `python tools/holes.py`
is at **33 sites**; item 16's one remaining site is the `CallArgs::FirstClassCallable`
arm (`crates/mwl-ir/src/lower/call.rs:85`), which is `mwl-ir` gap 1 and not item 16's.

`verify.py` 6 of 6 green — conformance **588**, differential **164**, 1629 unit tests.
`tools/leak-check.sh` green over a fixture with a borrowed spread subject, a freshly-built
one, two spreads in one call and a throwing variadic callee.

Three facts worth not re-deriving, all recorded where they belong rather than here:
the placement rule and its ownership are `lower_call_args`/`lower_variadic_tail`'s own doc
comments; the spread's key rule is ADR 0007 § 5 and its single PHP divergence is § 7
**row 12**; and a variadic parameter's *body* binding is `array<T>` while its recorded
signature type stays the element type — that split is commented at both binding sites
(`crates/mwl-types/src/check.rs:396`, `crates/mwl-ir/src/lower/mod.rs:688`) and was a live
bug this session, not a design change.

## Next group

**Item 7 — `$x++`/`--$x`, and the compound forms that inherit the same hole.** One file
set: `crates/mwl-ir/src/lower/stmt.rs` throughout, with the expression-position half in
`crates/mwl-ir/src/lower/expr.rs`. The item's own text (`tools/holes.py --item 7`) names
the two prerequisites, and they are the first slice rather than a preamble to it.

- [ ] **Split the target's *address* out of `lower_reassignment`** — `crates/mwl-ir/src/lower/stmt.rs:415`
      (`lower_reassignment`), `:706` (`is_reevaluable_target`), `:331`. `f()->count += 1` must
      evaluate `f()` once where the rewrite reads it twice, so a re-evaluable target is staged
      through `Lowering::staged_targets` (the playbook bullet on it is the whole mechanism) rather
      than lowered twice. `mwl-ir` gap 16.
- [ ] **`$x++`/`--$x` in statement position**, over a plain local, a compile-time-known property and
      an array element — same file, `crates/mwl-ir/src/lower/stmt.rs:920`. The increment's `1` has no
      source span, so it cannot be desugared into an `ExprKind::Int` the way every other compound
      form is: emit the constant directly, which is the second thing the item says to split out.
- [ ] **`$x++`/`--$x` in expression position**, where pre- and post- differ in the value the
      expression yields — `crates/mwl-ir/src/lower/expr.rs:258` is the arm that refuses an `Assign`
      as an expression today, and it is the same arm a `$n = $m = 0` would need.

## Backlog
- Item 20, `foreach (… as &$v)` — one site, `crates/mwl-ir/src/lower/control.rs:712`, its own file set.
- Item 19's three `&$x`-in-a-closure/generator sites — `lower/closure.rs:160`, `lower/generator.rs:155`/`:470`.
- Item 4 (bitwise) 2 sites and item 6 (`<=>`) 1 — `docs/agent/loop-goal.md` items 4 and 6.
- `mwl-codegen/src/ty.rs:116`/`:121` are unattributed by `holes.py` and no item names them.
- ADR 0007 § 5's remaining keyed-literal divergence: a positional element after an explicit
  `int`-looking key numbers from its own position — the third slice of the group just finished, untaken.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
