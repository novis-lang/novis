# Handoff

## State

**Goal 10 — stage 3 is half landed: a `fn` literal answers its own signature.** `check_fn_literal`
(`crates/nvs-types/src/expr/calls.rs:1564`) interns `Ty::CallableSig` from the parameters' lowered
types and the return type it already computed, so stage 2's lattice in
`crates/nvs-types/src/expr/assign.rs:60` finally has a source spelling that reaches it: the State
block's `E0401` on `callable(int): string $r = fn (int $n): string => "n";` is gone, prefix arity and
both variance directions are pinned from source in `crates/nvs-types/tests/callable.rs`, and a literal
still satisfies bare `callable` because every signature does. Nothing regressed: `ExprInfo::Closure`
still carries the class, the captures and the return type, and both binders that read a literal —
`Core\Arr::map`'s `Ty::CallableTo` and `Core\Task::all`'s `Ty::CallableShapeTo` — read it from there
rather than from the argument's type.

**What is left of stage 3 is not a `nvs-types` change first.** `rule:types/callable-literal-inference`'s
other half needs `fn ($u) => $u->name` to *parse*, and it does not: `parse_param` requires a type on
every parameter in the language. So the remaining slice is a grammar relaxation followed by threading
the expected type in, and the playbook bullet above is the trap that cost this session a test.

**The inherited wip commit is verified now.** `4090a4206` — session 0003's unwrapped
`crates/nvs-server/src/serve.rs` slice, a `run_the_core` helper that drives a core past a
`run_until_idle` that read a stale wake as idle — is left standing and went through this session's
`verify.py` with the rest.

The design is settled and is not re-derived: `rule:types/callable-signature`, `-arity`, `-variance`,
`-literal-inference`, plus the goal's own § *Standing decisions*.

## Next group

**The rest of stage 3: an unannotated parameter** — file set:
`crates/nvs-syntax/src/parser/expr.rs`, `crates/nvs-types/src/expr/calls.rs`,
`crates/nvs-types/src/expr/mod.rs`. Take both; the second is unwritable without the first.

- [ ] **A closure's parameter may omit its type.** `parse_param`
      (`crates/nvs-syntax/src/parser/expr.rs:1688`) reports `E0101` for a missing type on every
      parameter list alike; a closure's list — `parse_params`
      (`crates/nvs-syntax/src/parser/expr.rs:1670`), reached from the two `fn` forms at
      `crates/nvs-syntax/src/parser/expr.rs:1743` and `:1831` — leaves `Param::ty` `None` instead, and
      every other list keeps the refusal. `rule:types/callable-literal-inference`.
- [ ] **The expected type reaches the parameters.** `check_fn_literal`
      (`crates/nvs-types/src/expr/calls.rs:1564`) takes the expected `TypeId` its caller already holds
      (`crates/nvs-types/src/expr/mod.rs:631`) and fills each unwritten parameter from that type's
      corresponding position, `mixed` where there is none; a written parameter is unchanged and stays
      `rule:types/callable-variance`'s to check. `rule:types/callable-literal-inference`.

## Backlog

- Stage 4: `CoreTy::CallableTo` and `CoreTy::CallableShapeTo` become ordinary signatures — ADR 0136.
- `examples/typed-callable.nvs`, the goal's acceptance fixture — `docs/agent/loop-goal.toml`.
- `Core\Arr::map($users, fn($u) => $u->name)` typing `$u` as `User` — `rule:types/callable-literal-inference`.
