# Handoff

## State

**M4 — language completeness.** The property-write panic (`crates/mwl-ir/src/lower/stmt.rs`,
the `PropertyAccess` arm of `lower_store`) and its read-side twin
(`crates/mwl-ir/src/lower/expr.rs`, `lower_property_access`) both have **no reachable target
left**. The proof's only home is `mwl_types::expr::members::check_property_member`'s doc
comment: that function is the sole decider of what goes in the typed-expression table, and it
now records an entry for every access it returns from and refuses the rest.

Two shapes reached those panics and neither does now:

- **A computed member name** — `$obj->$name`, `$obj->{$expr}`, and both spellings in front of
  a call's parentheses — is `E0235` at `Parser::parse_member_name`, the same place and the same
  band `$$name`'s `E0202` is refused in. ADR 0014 § 5's body now carries the decision: the
  runtime-throw half survives for the two ways a name genuinely arrives late (a reflection
  get/set, and ADR 0036 § 4's erased receiver, where the name *is* written out), and a computed
  expression is not one of them.
- **An undeclared property on a `Core` class or the reserved exception tree** was excused from
  `E0405` by the class *kind*: nothing diagnosed and nothing recorded. Both excuses are gone —
  the exception tree's own properties are in `env.signatures` (`$e->message` resolves through
  the same call) and no `Core` class declares an instance property at all.

The item's stated gap — "ADR 0036 § 4's erased half still does not lower" — **did not exist**;
that half landed earlier and is verified working. The playbook bullet this session added is
about believing a panic's own account of itself.

## Next group

**The two remaining write-path panics that name an erased or unrecorded receiver.** The file
set: `crates/mwl-ir/src/lower/mod.rs`, `crates/mwl-ir/src/lower/stmt.rs`,
`crates/mwl-types/src/expr/assign.rs`, `tests/conformance/lang/`.

- [ ] **`mod.rs:2002` — an array-index assignment whose base is a property with no recorded
      declaring class.** Its message already argues an erased receiver cannot be what put it
      there (`check_write_target` refuses that as `E0480`,
      `crates/mwl-types/src/expr/assign.rs:501`), so the open question is what *else* can: the
      `let Some(ExprInfo::Property { .. })` binding takes neither `HookedProperty` (`E0478`,
      `assign.rs:484`) nor `ShapeProperty`. Enumerate the arms rather than trusting the
      message — see this session's playbook bullet. Anchors:
      `crates/mwl-ir/src/lower/mod.rs:1999`, `crates/mwl-types/src/expr/assign.rs:483`.
- [ ] **`stmt.rs:1245` — an intermediate level of a nested element write with no recorded
      element type** (`Lowering::row_ty_of`). Its message names a base erased to `mixed`, an
      unresolved array; ADR 0007 § 5 owns the separation the level is being asked for. Same
      two exits as the slice above: a diagnostic that names the rule, or the lowering. Anchors:
      `crates/mwl-ir/src/lower/stmt.rs:1243`, and the `assert!` on `row_ty == Ty::Array` at
      `crates/mwl-ir/src/lower/stmt.rs:1255` is the second half of the same question.

## Backlog

- `mwl-ir` gap 21 (`crates/mwl-ir/src/lib.rs:535`) reads stale: `object $o = $obj;` lowers and
  runs today, so the gap text and its "one line arm" plan need re-checking or deleting.
- `crates/mwl-ir/src/lower/expr.rs:3441` — the instance-call panic still names "a `mixed`, a
  union or a scalar receiver, which the checker does not yet refuse". `E0235` closed only its
  computed-name half.
- ADR 0007 § 2's `array<T> as array<U>` conversion row still panics `mwl-ir`
  (`lower/expr.rs:877`), which is what keeps several `Core` refusals unreachable from source —
  `docs/agent/playbook.md` § *Writing a test case* carries the worked cases.
