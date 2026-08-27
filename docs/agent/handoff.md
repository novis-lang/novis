# Handoff

## State

**M4 — ADR 0007 § 6's checked way out of `mixed` runs, and every object target
that cannot be checked is refused where it is written.** `$m as Plain` tests the
value's runtime class and throws on a miss; `$m as object`, `$m as callable` and
`$m as Core\Uri` are `E0711`. `Lowering::convert`'s catch-all now has no object
target left at all.

- **The downcast needed no new machinery.** `InstKind::InstanceOf` already takes
  a `Ty::Tagged` subject and already answers `false` for a non-object tag, so
  `Lowering::lower_checked_downcast` (`crates/mwl-ir/src/lower/expr.rs:4934`) is
  that test, a `Terminator::Throw` on the false edge and one free
  `InstKind::Untag` on the true one. It lives in `lower_conversion`'s `None` arm
  rather than in `convert`, whose `cur` is by value and so cannot branch. The
  class name comes from `lower::closure`'s `declared_class`, now `pub(super)`:
  one question, one answer, shared with the closure parameter's entry check.
- **The refcount edge is the free row's, split across two edges.** `Untag`
  relabels, so the object shares the tagged operand's reference: a borrowed
  operand is retained on the way out, a fresh one transfers, and the *false*
  edge releases a fresh one before the throw rather than abandoning it there.
  Valgrind-clean over a fixture that converts and fails to convert 200 times.
- **The other half is a refusal, and it is the safe option under a real
  tradeoff.** `object`, a shape, a `callable` and a `Core` class name no
  descriptor to compare against, so the conversion could only assert a tag it
  cannot verify. `mwl_types::expr::operators`' `reject_untestable_object_target`
  refuses those from a non-object operand (`E0711`); the cost is that there is
  no spelling at all for getting a `Core\Uri` out of a `mixed`, which is the
  same shape as `mwl-ir` gap 12 and is in the backlog rather than lost.
  `Core\Html\Markup` is exempt: ADR 0024 § 5's row is `quals.rs`' to decide.

## Next group

**The last row of `Lowering::convert`'s catch-all, then the erased-operand half
of `mwl-ir` gap 12.** The file set: `crates/mwl-ir/src/lower/expr.rs`,
`crates/mwl-ir/src/ir.rs`, `crates/mwl-runtime/src/helpers.rs`,
`tests/conformance/lang/`.

- [ ] **`$xs as array<U>`** — ADR 0007 § 2's O(n) element walk, the one row of
      that table that is not a single helper call, and now one of the two
      shapes `convert`'s catch-all still names
      (`crates/mwl-ir/src/lower/expr.rs:1032`, the `_ =>` arm at the end). The
      open design question is how the target element type travels: helper
      arguments are stored as `mwl_runtime::Value`s
      (`crates/mwl-codegen/src/emit.rs:1802`), so a tag code as an integer
      argument is the shape that fits, and `Helper::TaggedToBytes`
      (`crates/mwl-ir/src/ir.rs:1639`) is the nearest existing row to copy.
      `crates/mwl-runtime/src/helpers.rs:985` (`stringify`) is how a runtime
      walk over a tagged value is written today. A `U` that names a *class* has
      no answer and should take the same `E0711` refusal this session added.
- [ ] **A `Core` object behind a `mixed` renders** — gap 12's residual.
      `mixed $m = Core\Uri::parse(…); echo $m;` throws "does not implement
      `Stringable`" because `mwl_runtime::stringify`
      (`crates/mwl-runtime/src/helpers.rs:985`) reads a compiled method table
      and `mwl-runtime` sits below `mwl-stdlib`, so it cannot ask
      `mwl_stdlib::registry::class_renders`
      (`crates/mwl-stdlib/src/registry.rs:876`). The two candidate shapes are in
      that gap's own text; `mwl_runtime::ctx`'s `is_carrier`
      (`crates/mwl-runtime/src/ctx.rs:179`) is the precedent for a list the
      runtime holds about `Core` classes without reading the registry.

## Backlog

- A `Core` class target of `as` has no checked spelling at all (`E0711`), so
  nothing gets a `Core\Uri` out of a `mixed` — same shape as gap 12, owned by
  `crates/mwl-diagnostics/src/lib.rs`' `E_UNTESTABLE_CONVERSION_TARGET`.
- ADR 0024 § 5's `string as Core\Html\Markup` is a checked row in `mwl_types`
  and still panics in `mwl-ir` — M7, when `Core\Html` exists.
- `python tools/holes.py`, `python tools/loop.py --list` and
  `python tools/check-migration.py` are the live worklists; never re-derive one.
- ADR 0053 § 4's abandoned-generator `finally` — `docs/agent/loop-goal.md`
  § *Standing decisions* pre-authorizes the shape.
- `E04xx` is full at `E0499`; the `E07xx` band continues it and is at `E0711`.
