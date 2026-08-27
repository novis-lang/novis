# Handoff

## State

**M4 — language completeness.** `$fn(...)` lowers: one `Helper::CallClosure`
(`mwl_ir::lower::Lowering::lower_closure_call`, `crates/mwl-ir/src/lower/call.rs:643`) carrying the
closure at `args[0]` and its arguments after it, which is `mwl_runtime::mwl_call_closure`
(`crates/mwl-runtime/src/closure.rs:169`) and from there the same `call_closure` every `Core` member's
callback already takes — one body, no second convention. It is **the one variadic helper**: its arity
belongs to the call site, so `mwl-codegen` passes the count beside the argument slot
(`Signatures::helper_variadic`, `crates/mwl-codegen/src/lib.rs:589`) where every other helper's arity is
a literal in its `mwl_helper!` expansion. The result is `Ty::Tagged`, because ADR 0031 § 1 leaves the
checker `mixed` as its only answer; arguments are borrowed, the treatment every helper's are given.
Extra arguments are trimmed as a `Core` callback's are; too few is a catchable `LogicError` rather than
the engine fault a native caller gets, since no checker could have counted them.

**A closure's declared parameter types are checked by nobody, and that is a priority-1 hole.** It is
older than this lowering and reachable from safe MWL without it — `Core\Arr::map($ints, fn (string $s)
...)` over an `array<int>` reads an `int` payload as a pointer — so `$f(...)` widens who can reach it,
not whether. `mwl_runtime::closure`'s module doc owns it and states the fix; it is the next group.

`verify.py` 6 of 6 green — conformance **617**, differential 173. `tools/leak-check.sh` clean over two
fixtures that carry a captured `string` and a `string` argument through a direct closure call and a
throwing one.

## Next group

**A closure carries what its parameters are, and the call checks them.** The four files are one file
set: `crates/mwl-ir/src/lower/closure.rs`, `crates/mwl-ir/src/lower/mod.rs`,
`crates/mwl-runtime/src/closure.rs`, `crates/mwl-codegen/tests/closures.rs`.

- [ ] **A closure object records its parameter tags beside its arity.** One more reserved field
      written from the declared types at the literal, exactly as `FN_ARITY`
      (`crates/mwl-ir/src/lower/mod.rs:2642`) is today, in `lower_closure`
      (`crates/mwl-ir/src/lower/closure.rs:119`). Every capture slot moves up by one, so
      `crates/mwl-codegen/tests/closures.rs:27` is the test that says whether the layout still agrees.
- [ ] **`call_closure` compares each argument's tag against them and throws.**
      `crates/mwl-runtime/src/closure.rs:97` is where the arity is already read and the slice already
      trimmed; the comparison goes beside it, answering `ThrownClass::Logic` the way
      `mwl_call_closure`'s arity check at `crates/mwl-runtime/src/closure.rs:169` does. One tag
      comparison per argument on the callback path is priority 3 spent for priority 1 — AGENTS.md's
      ordering names that direction.
- [ ] **A `.mwlt` case pinning both sides, and both callers.** A matching call runs and a mismatched
      one throws, asked once of `$f(...)` and once of `Core\Arr::map` so the two callers agree —
      `tests/conformance/lang/a-closure-is-called-through-the-variable-holding-it.mwlt` is the file to
      extend rather than a second one.

## Backlog

- `mwl-ir`'s known gaps 15 and 16 no longer describe the tree (`crates/mwl-ir/src/lib.rs:347`): `$x++`
  and `--$x` lower now, and `<=>` over a scalar is what is actually left.
- A `name:` or `...` argument to `$f(...)` panics rather than diagnosing — gap 8's checker half first,
  as `docs/agent/loop-goal.md` § *Standing decisions* orders it.
- The first-class callable spelling `$f(...)`/`Class::method(...)` still panics `mwl-ir`;
  `docs/agent/playbook.md` § *Writing a test case* has the shape cases use instead.
- `array<T> as array<U>` (ADR 0007 § 2) does not lower, which is what blocks a case from reading past
  the first level of an `array<mixed>`.
- `python tools/holes.py`'s ranked items overstate what is open; `--cases` is the half to trust.
