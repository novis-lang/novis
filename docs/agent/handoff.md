# Handoff

## State

**Goal `m4-refusals` — Stage 5's `throw` half is closed; its `clone` half is the next group.**
`python tools/holes.py` reports **7** refusal sites, `UNATTRIBUTED: 0`, **13** guarded, and
`crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **7** to match.

- `crates/nvs-ir/src/lower/exception.rs:87`'s `guard_throwable` is the tagged `throw` operand's two
  questions, asked in front of the raise: `InstKind::TagIs` against `Ty::Object`, then
  `InstKind::InstanceOf` against `Throwable` on the untagged value. Both failures raise a
  `LogicError` through `raise_logic_error` (`:168`) carrying PHP's own wording — `Can only throw
  objects` and `Cannot throw objects that do not implement Throwable`, verified against PHP 8.5.9.
- A `Ty::Object` operand pays neither check and reaches `Terminator::Throw` directly; its class is
  the checker's own answer. The remaining assert is an engine invariant naming
  `nvs_types::expr::members::reject_unthrowable`, so `holes.py` no longer counts the site.
- The stage's first acceptance check still fails on its two `clone` cases, which the next group
  writes. Nothing is blocked.

## Next group

**Stage 5: `clone` through a tag** — one file set: `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/emit.rs` and `crates/nvs-runtime/src/object.rs`.
`docs/agent/loop-goal.md` § *Stage 5* is the spec and `rule:classes/clone-is-shallow` the operand rule.

- [ ] **A tagged `clone` operand is untagged behind one check** — `crates/nvs-ir/src/lower/expr.rs:5295`,
      whose assert is the last `Ty::Object` claim on that path. The checker passes `mixed`, `object`
      and every union on purpose (`crates/nvs-types/src/expr/members.rs:429`), which the goal's
      § *Standing decisions* settles as a shape to lower rather than refuse.
      `crates/nvs-ir/src/lower/exception.rs:87` is the same branch one operator over — take its shape
      rather than writing a second one.
- [ ] **The refusal's message needs a type name, and that is the decision in this group** —
      `crates/nvs-runtime/src/object.rs:2809`. PHP 8.5.9 answers `clone` over a non-object with
      `clone(): Argument #1 ($object) must be of type object, <type> given`, and `<type>` is PHP's
      spelling — see the playbook bullet for why `tag_name` is not it. Two ways: render PHP's
      spellings in a new `Helper` and build the string with `InstKind::Concat`, or put the whole guard
      behind a fallible `nvs_value_clone` taking a `Value` beside today's infallible
      `nvs_object_clone` (`RuntimeSig::PtrToPtr`, `crates/nvs-codegen/src/emit.rs:743` picks it) and
      let the runtime word it. The second is `rule:types/erased-member-access`'s own shape — the
      runtime checks the tag where `InstKind::SlotGet` already does.
- [ ] **The stage's three remaining cases** — `docs/agent/loop-goal.toml:9627` and `:9643` are the two
      stage-5 checks that name them: `tests/conformance/class/cloning-through-a-nullable-clones-the-object-it-holds.nvst`,
      `tests/conformance/class/cloning-null-through-a-nullable-throws-an-error.nvst` and
      `tests/differential/class/cloning-null-matches-phps.nvst`.
      `tests/differential/error/throwing-null-and-a-non-throwable-match-phps.nvst:1` is the shape to
      copy, and `rule:classes/clone-is-shallow` is what each `--TEST--` line cites.

## Backlog

- `throw $c` and `clone $c` over a `class<T>` reach lowering as `Ty::ClassDesc`: `can_hold_an_object`'s
  `_ => true` arm (`crates/nvs-types/src/expr/members.rs:418`) admits `CheckedTy::ClassRef`, which no
  rule says is throwable or cloneable — owner: this goal, after stage 5.
- A `Ty::Object` `throw` operand gets no tree check, so `object $o = new Plain(); throw $o;` raises an
  object no `catch` matches — `crates/nvs-ir/src/lower/exception.rs:87` guards the tagged half only.
- `InstKind::TagIs { repr: Ty::Object }` compares one tag byte (`crates/nvs-codegen/src/emit.rs:902`),
  so a `Tag::Closure` payload answers `false` though `rule:types/closure-literal` makes a closure an
  object — `$x is object` reads the same way.
- `tests/conformance/reject/goto-is-refused-at-the-keyword.nvst` cites no rule, `goto` being refused by
  no fragment under `docs/rules/` — owner: `docs/rules/statements/`.
