# Handoff

## State

**M4's Stage 8, plus the acceptance gate's own open item.** The tree is at **866 conformance plus
189 differential**, all green. Nothing is blocked.

The guard-name debt is **14 unresolved of the 132 named test entries** `loop-goal.toml` holds, down
from 17-as-recorded (really 18 — see below). Stage 7 is **finished**: all three names in
`nvs-stdlib (the assertion roster)` were cause 2, so that whole `[[check]]` block is **gone** and
four `.nvst` cases joined the `nvs-suite` `cases` list instead (each run green first) — § 5's ledger,
§ 5's catchable `Core\Test\Failure`, and the two machine halves of § 22's three renderings beside the
human one already listed. ADR 0079's surface is landed, not owed; the toml comment claiming
otherwise is corrected and the new playbook bullet is why.

Stage 4's undecided line is **decided and written**: `the_object_top_type_erases_to_the_pointer_a_
class_does` was cause 3, and is now a Rust test of that exact name at
`crates/nvs-ir/src/lower/tests.rs:1369` — an agreement over the three spellings `erase_checked_ty`
answers `Ty::Object` for, asserted on `Function::params` rather than in a snapshot.

Both of the debt file's counts are now **derived off the tree** by the pass its header describes,
which is what caught the old **17** being one short of the eighteen lines then unticked.

## Next group

**The five `nvs-types (targets and refusals)` lines** — the last block where the triage is cheap and
the fix may already be on disk. Shared file set: `crates/nvs-types/src/expr/assign.rs`,
`crates/nvs-types/src/defaults.rs` + `consts.rs`, `crates/nvs-types/src/expr/operators.rs`, plus
`docs/agent/guard-name-debt.md` and `docs/agent/loop-goal.toml` for the reconciliation, and
`crates/nvs-diagnostics/src/lib.rs` only if a name turns out to owe a new code (next free `E0739`).

- [ ] **`a_nullsafe_assignment_target_is_a_compile_error`** — the refusal is **already landed** at
      `crates/nvs-types/src/expr/assign.rs:493`, with its rationale in the doc comment at
      `assign.rs:433`. So this is cause 1 or 2, not the new-diagnostic slice the standing decisions
      pre-authorize: find the test naming it (`grep -rhoE "fn [a-z_]+" crates/nvs-types/tests`) or
      the `.nvst` case, and move or rename it.
- [ ] **`an_element_write_through_a_hooked_property_is_a_compile_error`** — same file
      (`assign.rs`), same standing decision (loop-goal.md, "indirect modification of overloaded
      property"); check whether the refusal exists beside the nullsafe one before writing anything.
- [ ] **The ADR 0066 pair** — `a_nullable_conversion_that_cannot_fail_is_a_compile_error` and
      `..._that_does_not_exist_is_a_compile_error`, both § 3's target rule, whose implementation is
      `crates/nvs-types/src/expr/operators.rs:1186` and its neighbours at `:98`/`:108`.
- [ ] **`a_property_default_accepts_every_compile_time_constant`** — `defaults.rs` folds a default
      to a `ConstArg` at signature collection; `consts.rs:257` (`fold_const`) is what decides what a
      compile-time constant *is*, so the guard is an agreement between those two.

## Backlog

- `an_inline_producer_releases_its_value_on_the_throw_path` and
  `a_transferred_argument_is_released_when_a_later_one_throws` — cause 3, real refcount work;
  `nvs_ir::lower::Lowering`'s owned-temporaries field doc owns what is left.
- `an_array_conversion_walks_its_elements` — blocked on `array<T> as array<U>`, which panics
  `nvs-ir` at `crates/nvs-ir/src/lower/expr.rs:877`. Several playbook bullets wait on this row.
- The four cause-3 lines in Stages 5/6 and the two nvs-syntax parse shapes — each is a test owed,
  reasons per line in `docs/agent/guard-name-debt.md`.
- `every_refusal_is_a_diagnostic_or_decided` — Stage 8's load-bearing guard, red on its merits while
  `python tools/holes.py` still reads standing refusal sites.
