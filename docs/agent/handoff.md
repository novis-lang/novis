# Handoff

## State

**Goal 10 — `Core\Arr::map` and `::filter` are the first two rows that spell their callback's
signature, and the checker types a closure literal from them.**

- **A `fn` literal at a parameter written in variables is checked in its own round of the binding
  pass** (`crates/nvs-types/src/expr/args.rs:1289`), between the written arguments and the untyped
  literals: its expected type is the parameter with the bindings so far put in, so
  `Core\Arr::map($users, fn($u) => $u->name)` types `$u` as `User`, and the signature it reports then
  binds `U` before `MethodSig::substituted` would have collapsed it to `mixed`.
  `rule:types/callable-literal-inference`.
- **A first-class-callable reference carries the referenced member's own signature**
  (`crates/nvs-types/src/expr/calls.rs:802`), so `Core\Arr::map($xs, Core\Math::abs(...))` binds `U`
  from `abs`'s return type. Every declared parameter is in it, optional ones included — the
  conservative side of `rule:types/callable-arity`'s prefix match.
- **Bare `callable` no longer satisfies those two rows**, which is `rule:types/callable-signature`'s
  lattice and not an accident. Eight conformance cases asked the *runtime* callback check through
  `filter`/`map`; they now ask it through `Core\Arr::any` and `::mapKeys`, whose callbacks are still
  bare. The playbook bullet above is that trap.

`CoreTy::CallableTo` still carries `Core\Task::map`, `Core\Cli`'s two and `Core\Db`'s two;
`CallableShapeTo` still carries `Task::all`. Conformance is 1571 green.

## Next group

**Stage 4's other half: no `Core` row leaves its callback unspelled** — file set:
`crates/nvs-stdlib/src/arr.rs`, `crates/nvs-stdlib/src/task.rs`, `crates/nvs-stdlib/src/cli.rs`,
`crates/nvs-stdlib/src/db/registry.rs`, `docs/spec/01-core-library.md`, and the conformance cases the
first item strands. Take them in order; the second is the first one's fallout and cannot be split
from it.

- [ ] **Every `Core` callback parameter spells its signature, and `CoreTy::CallableTo` is retired.**
      The variant is `crates/nvs-stdlib/src/registry.rs:453`'s neighbour and its five rows are
      `crates/nvs-stdlib/src/task.rs:148`, `crates/nvs-stdlib/src/cli.rs:318`,
      `crates/nvs-stdlib/src/cli.rs:327`, `crates/nvs-stdlib/src/db/registry.rs:523` and
      `crates/nvs-stdlib/src/db/transaction.rs:732`; the bare `CoreTy::Callable` rows start at
      `crates/nvs-stdlib/src/arr.rs:125`. `docs/agent/loop-goal.toml`'s stage 4 names the two tests
      (`no_registry_row_names_a_callable_binding_site_variant`,
      `every_callback_parameter_declares_its_signature`). `rule:types/callable-signature`, and
      `rule:core-api/reference-card` puts the spec table in the same slice.
- [ ] **The cases that reach the runtime callback check lose their last bare-`callable` member.**
      `tests/conformance/core/arr-a-mixed-or-nullable-callback-parameter-is-unchecked.nvst:31`,
      `tests/conformance/core/arr-a-callback-enum-parameter-is-its-backing-integer.nvst:66` and
      `tests/conformance/core/arr-a-callback-float-parameter-widens-an-int-and-stops-at-2-53.nvst:74`
      each ask `Core\Arr::any` what a wrongly-declared parameter does at run time. Once no row takes
      a bare `callable`, the question moves to a direct `$fn(...)` call, which is the one caller
      `rule:types/callable-is-a-closure` leaves dynamic — not to `--EXPECTF-ERROR--`, which would
      assert the refusal a third time and drop the runtime answer.

## Backlog

- Stage 5: `Core\Task::all` binds its shape structurally and `CallableShapeTo` retires —
  `docs/agent/loop-goal.toml`.
- Stage 6: a proven call site emits no `check_param_tags` — `docs/agent/loop-goal.toml`,
  `a_proven_callable_call_site_emits_no_param_tag_check`.
- Stage 6: `examples/typed-callable.nvs` and `tests/conformance/types/callable/` — the goal's own
  acceptance fixtures, both still absent.
- `nvs_types::expr::calls`' `check_fn_literal` still refuses an unannotated parameter past the
  expected signature's arity (`E0808`); nothing asks for more.
