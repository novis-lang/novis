# Handoff

## State

**Goal 10 — stage 6 is whole, and it is the goal's last stage: a call site whose callee carries a
written signature is checked where it is written and pays nothing at run time.**

- **The proof is the checker's, and it did not exist before this session.** A call through a
  `Ty::CallableSig` callee now checks each argument against the parameter it fills and answers the
  signature's own return type — `crates/nvs-types/src/expr/calls.rs:1428`, reached from the
  `ExprKind::Call` arm at `crates/nvs-types/src/expr/mod.rs:415`. The count is exact where the value's
  own arity is not (`E0809`, and `rule:types/callable-arity` gained the sentence saying why).
- **Spending it is one helper row.** `nvs_ir::ir::Helper::CallClosureProven` →
  `nvs_runtime::closure::nvs_call_closure_proven`, which is `call_closure` with
  `TagCheck::Proven` and nothing else changed. The lowering reads
  `ExprInfo::CallThroughSignature` off the call's own span
  (`crates/nvs-ir/src/lower/call.rs:@lower_closure_call`) and coerces each argument into its
  parameter's representation itself — that coercion is not an optimisation, it is the
  `int`-into-`float` widening `check_param_tags` used to perform.
- **Three shapes deliberately keep the dynamic path**: bare `callable`, a `...` argument (the count
  is a run-time fact, so no argument has a parameter to be checked against), and a self-name
  recursive call. Both closure metadata slots stay on every closure object, guarded by
  `a_closure_object_still_carries_both_metadata_slots`.
- **Valgrind is green** over a fixture that runs a proven call with `string` arguments and a
  narrowed `string` result 5000 times — the leg the goal's stage 6 § 3 makes non-optional.
- `examples/typed-callable.nvs` runs and prints the goal's four lines. The stage-6 `nvs-suite`
  check named `tests/conformance/types/callable`, a two-level layout the corpus never adopted
  (the playbook bullet owns that trap); it now runs `tests/conformance/lang/` and names its two
  cases.

Conformance is 1572. Verify: 7 of 7 green.

## Next group

**The shapes the proof does not reach yet, and the rulebook catching up** — file set:
`crates/nvs-types/src/expr/mod.rs`, `crates/nvs-types/src/expr/calls.rs`, `docs/rules/types.json`.
Take them in order; the first is the only one with a soundness edge in it.

- [ ] **A self-name recursive call is checked against the closure's own parameters.** `fact($n - 1)`
      inside `fn fact(int $n): int` returns `fn_self.ret` with its arguments checked against nothing
      (`crates/nvs-types/src/expr/mod.rs:402`), so a wrong argument type reaches
      `nvs_runtime::closure`'s tag check rather than the diagnostic every other callable position
      now gives. The closure being written *is* the signature, so there is a parameter list in hand.
      `rule:types/closure-self-name`, `rule:types/callable-signature`; a conformance case beside
      `tests/conformance/lang/a-call-through-a-written-signature-is-checked-where-it-is-written.nvst`.
- [ ] **A `...` argument through a written signature answers the declared return type.** The count
      is a run-time fact so the *arguments* stay the runtime's business, but what the callee returns
      does not depend on them — today the whole call falls back to `mixed`
      (`crates/nvs-types/src/expr/calls.rs:1441` is the guard that hands it back).
      `rule:types/callable-signature`.
- [ ] **ADR 0136's four rules move from `designed` to `shipped`, and gain their guards.**
      `docs/rules/types.json:413` is `types/callable-signature`, still `"status": "designed"` with an
      empty `guardedBy`, and `callable-arity`/`callable-variance`/`callable-literal-inference` are
      the three below it. All four now run end to end. `python tools/rules.py --render` after,
      and `docs/novis.md` picks them up (`tools/reference.py` filters to `shipped`).

## Backlog

- The driver's acceptance run decides whether goal 10 is met: every stage-6 check now has its
  artefact on disk (`docs/agent/loop-goal.toml` stage 6).
- `nvs_call_closure_array` has no proven twin on purpose — `crates/nvs-runtime/src/closure.rs`'s
  own doc says why, and a later measurement is what would change it.
- `NoParameterList::Callable`'s refusal text for a `name:` argument still explains itself with
  "`callable` carries no parameter list", which is false for the signature spelling
  (`crates/nvs-types/src/expr/calls.rs:@report_args_with_no_parameter_list`).
