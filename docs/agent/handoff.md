# Handoff

## State

**Goal 10 — the checker's proof reaches every callable position ADR 0136 names, and the rulebook
says so: all four of the record's rules are `shipped` with their guards.**

- **A self-name recursive call is checked against the closure it names.** `FnSelf` carries the
  literal's own parameter list beside its return type (`crates/nvs-types/src/lib.rs:346`), and
  `calls::check_self_name_args` holds each argument to the parameter it fills, with `E0809` on the
  count. It is the same list a written `callable(int): string` gives, read off the literal being
  checked rather than off a value's type, so `rule:types/closure-self-name` gained the paragraph
  saying what a call through the name is held to.
- **Nothing is recorded for `nvs-ir` there.** A self-call still reaches the closure through the
  dynamic path and its tag check; the proof buys the diagnostic, not a helper row.
- **A `...` argument through a written signature answers the declared return type** rather than
  falling back to `mixed` (`crates/nvs-types/src/expr/calls.rs:1441`). The count is the spread
  subject's own run-time length, so the arguments stay `nvs_runtime::closure`'s business and no
  `ExprInfo::CallThroughSignature` is recorded — what the callee answers with does not depend on
  them.
- **`E0809` names its subject.** `Signature::Written` keeps the wording the existing conformance
  case pins; `Signature::SelfName` says "this closure" and why a shorter list has no wider type to
  be matched against.
- **The driver's earliest failing check is stage 2, and both of its checks name tests that exist
  under other names** — the five parsing claims are three tests at
  `crates/nvs-syntax/src/parser/tests/ty.rs:704`, and the assignability claims are the run at
  `crates/nvs-types/tests/callable.rs:70`. The check is the specification; the next group renames
  and splits to it.
- **`session.py --wrap` refused this session's tail and these commits were made by hand**, staging
  own paths only: `rules.py --check` fails on three untracked `docs/rules/observability/` fragments
  and `docs/decisions/0148.md`, none of them this session's, and that gate is whole-tree. It clears
  itself once whoever is writing ADR 0148 declares those fragments in `observability.json`.

Conformance 1574. Verify: 7 of 7 green.

## Next group

**Stage 2's two acceptance checks, under the names they list** — file set:
`crates/nvs-syntax/src/parser/tests/ty.rs`, `crates/nvs-types/tests/callable.rs`. Nothing here is a
new feature: both stages run, and only the test names the goal fixed are missing.

- [ ] **The five `nvs-syntax` names in `docs/agent/loop-goal.toml:4631` exist.** Split
      `a_callable_type_carries_its_parameters_and_its_return_type`
      (`crates/nvs-syntax/src/parser/tests/ty.rs:704`) into
      `a_callable_type_parses_its_parameters_and_return`, `a_callable_type_with_no_parameters_parses`
      and `a_bare_callable_still_parses_as_the_opaque_atom`, and
      `a_callable_type_refuses_a_parameter_name_and_a_missing_return`
      (`crates/nvs-syntax/src/parser/tests/ty.rs:780`) into
      `a_callable_type_without_a_return_type_is_refused` and
      `a_callable_type_naming_a_parameter_is_refused`. `rule:types/callable-signature`.
- [ ] **The seven `nvs-types` names in `docs/agent/loop-goal.toml:4648` exist.** Five are renames of
      the run at `crates/nvs-types/tests/callable.rs:70`; two directions have no test at all —
      `a_callable_parameter_may_be_wider_than_the_slot_declares` and
      `a_callable_return_may_be_narrower_than_the_slot_declares`, the *accepted* half of
      `rule:types/callable-variance` — and `every_callable_signature_is_assignable_to_bare_callable`
      is the lattice top read from a signature-typed value rather than from a literal.
      `rule:types/callable-arity`, `rule:types/callable-variance`.

## Backlog

- Goal 10's stages 1-6 are whole; stage 2's names above are all that stands between the tree and the
  goal's acceptance list (`docs/agent/loop-goal.toml`).
- `E0450` and whole-body return-type inference stay refused — ADR 0136 § *Revisiting*, second entry.
- Optional and variadic parameters in a callable type stay refused — ADR 0136 § *Revisiting*, first
  entry.
- A self-name call passing a `...` is answered but not held to the parameter list, the same shape
  `check_call_through_signature` leaves to the runtime (`crates/nvs-types/src/expr/calls.rs:1475`).
