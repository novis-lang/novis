# Handoff

## State

**Goal `test-doubles`, stage 3's refusals are on disk and green.**
`nvs_types::conformance::check_double_answers` (`crates/nvs-types/src/conformance.rs:558`) compares the
shape a `Core\Test::double`/`partial` call answers with against the interface its type argument named,
hooked into the static-call arm at `crates/nvs-types/src/expr/calls.rs:420` — the one point where the
written type argument and the typed arguments are both in hand. Three codes in
`crates/nvs-diagnostics/src/lib.rs:3758`: `E0825` a method the interface requires and the shape leaves
out (`double` only — a `partial`'s `$real` answers the rest), `E0826` a field naming no method the
interface declares (both members, and a `private` interface method is named by it too), `E0827` a type
argument that resolved to something other than an interface.

`E0827` is a **deviation from the goal's stage 3 prose**, which planned to draw the existing code: `E0465`
says a type argument is not a class, which is false of the class this refusal names, and
`E_PROGRAM_TYPE_ARG_NOT_AN_INTERFACE` is the same demand with its own code at
`rule:programs/implementing`. A type argument that names no declaration at all still takes `E0465` alone.

Five `cargo test -p nvs-types --test testing` cases and the two `tests/conformance/reject/` cases the
stage's checks name all pass. `crates/nvs-stdlib/src/test.rs:262` now records that no checked program
reaches `nvs_abstract_method`'s fallback.

**Stage 3's third item is not built**: a closure's parameters and return are not yet checked against the
method it answers, so `{now: fn(): string => "x"}` at a `now(): int` is accepted here and left to
`nvs_runtime::closure`'s dynamic check. That is the next group.

## Next group

**Stage 3: the closure signatures** — one file set: `crates/nvs-types/src/conformance.rs`,
`crates/nvs-types/src/expr/assign.rs`, `crates/nvs-types/tests/testing.rs`,
`tests/conformance/reject/`.

- [ ] **Each closure is checked against the method it answers** — goal prose stage 3 item 3. The walk at
      `crates/nvs-types/src/conformance.rs:558` already resolves the `MethodSig` for every field it
      accepts; what it throws away is the field's own type, since `answered_methods`
      (`crates/nvs-types/src/conformance.rs:670`) collects names alone — return `(name, TypeId)` pairs
      and the comparison has both sides. The relation is `crates/nvs-types/src/expr/assign.rs:60`'s
      `is_assignable` over the field's `Ty::CallableSig`; a field typed bare `callable` carries no
      parameter list (`rule:types/callable-is-a-closure`) and is the accepted case, not a refusal.
      Decide there whether this is a fourth `E08xx` code or the argument-mismatch code the position
      already carries, and record the call in the code comment. `rule:testing/doubles`.
- [ ] **The reject case for it** — `tests/conformance/reject/`, shaped like
      `tests/conformance/reject/a-double-missing-a-method-is-refused.nvst:22`: `--EXPECT--` blank,
      `--EXPECTF-ERROR--` with `%A` under the message line, then `error: aborting due to 1 error`.
      `rule:testing/doubles`.

## Backlog

- Stage 4: `assertCalled`/`assertNeverCalled` and the method reference, which needs a new `CoreTy`
  variant — one edit in `registry.rs` and four outside it (playbook, *Writing Novis itself*).
- Stage 5: `assertCompletes` under the virtual clock — `docs/agent/loop-goal.md` § *Stage 5*.
- Stage 6: the rulebook — `rule:testing/doubles` and `rule:testing/interaction-after-the-fact` are still
  marked *designed, not yet shipped*, and neither names a guard test yet.
- The three `Test::assert*` ratchet keys in
  `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:26`, struck by stages 4 and 5.
- `nvs-server`'s `a_fleet_lease_is_renewed_while_its_run_is_in_flight` failed once beside the other test
  binaries and passes alone — a timing flake under load, not this work.
