# Handoff

## State

**M4's Stage 8, plus the acceptance gate's own open item.** The tree is at **867 conformance plus
189 differential**, all green. Nothing is blocked.

The guard-name debt is **9 unresolved of the 128 named test entries** `loop-goal.toml` holds, and
the `nvs-types (targets and refusals)` block is now **0 of 7** — closed. Both of the debt file's
counts stay derived off the tree by the pass its header describes, never carried forward.

This session's name was **cause 3, and the triage it asked for came back "a fold to widen"**: a
property default folded a literal and `[]` and nothing else, so the other two members of ADR 0046
§ 2's constant set — an enum case, another class's `const` — were `E0472`. Both are accepted now
(`nvs_types::defaults::const_reference_default`), at a *property* default only; the parameter half
is still a literal, and that module's own doc owns why. `E0472`'s wording moved from "literal" to
"constant" with it, which is why the pinned `.nvst` case and two playbook bullets changed.

## Next group

**The two remaining Stage 4/6 guard names, both cause 3.** Neither touches this session's files, so
this is a fresh file set: `crates/nvs-types/src/expr/calls.rs`, `crates/nvs-types/src/check.rs` and
`crates/nvs-types/tests/`, plus `docs/agent/guard-name-debt.md` and `docs/agent/loop-goal.toml` for
the reconciliation. Read the item's own `loop-goal.toml` comment, but check the crate before
believing it — the playbook bullet about a stale comment is there for a reason.

- [ ] **`an_implicit_constructor_is_held_to_zero_arguments`** — a class that declares no
      `constructor` still has one, and `new C(1)` must be an arity refusal rather than a silently
      dropped argument. `crates/nvs-types/src/expr/calls.rs:267`'s `infer_new` is the arm that
      resolves the constructor signature; find out whether it looks one up at all when the class
      declares none before deciding this is a test or a check.
- [ ] **`a_non_void_function_must_return_on_every_path`** — the *refusal* half. The definite-return
      walk already exists for the constructor obligation at
      `crates/nvs-types/src/ctor_init.rs:210`; what is missing is a non-`void` return type reaching
      it, which would be at `crates/nvs-types/src/check.rs:459`'s `check_method`. If no diagnostic
      exists, the band's next free code is in the orientation pack.
- [ ] **The parameter half of ADR 0046 § 2's constant set**, if the two above land cheaply — same
      set at `eval_param_default`, blocked only on `nvs_ir::lower::emit_const_arg` taking the
      position's IR type rather than the constant's, so an enum case does not arrive at a
      `Ty::Enum` slot as a `ConstArg::Int`.

## Backlog

- `an_array_conversion_walks_its_elements` — `array<T> as array<U>` panics `nvs-ir` at
  `lower/expr.rs:877`; nothing to guard until it lowers (`docs/agent/guard-name-debt.md`).
- A `uint` class constant above `i64::MAX` has no folded value, so it cannot be a property default
  even though the literal can (`crates/nvs-types/src/defaults.rs`'s module doc).
- A user-declared class constant's *declared type* is still unmodeled — `Class::CONST` infers
  `mixed` at every expression site (`crates/nvs-types/src/lib.rs`'s known gaps).
- Stage 7's remaining named cases, `python tools/loop.py --list`.
