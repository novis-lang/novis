# Handoff

## State

**ADR 0007 § 4's operand end is closed for a value that is not one.** A call
returning `void` used under any operator is `E0718` where it is written
(`nvs_types::expr::operators::reject_void_operand`, whose doc comment is the
rule's home), the three arithmetic prefixes included; `.` alone keeps `E0707`,
whose own roster already names a `void` call. The plan's `Open now` carries the
reasoning and the two `.nvst` cases that pin it.

- **The same value in a *condition* is still a panic**, and it is a hole of its
  own rather than part of this one: `if (V::nothing())` reaches
  `crates/nvs-ir/src/lower/convert.rs:567` with *"got Void"*. ADR 0035 makes a
  condition the one place a value is tested without `as`, and "a value" is what
  is missing — `nvs-types` has no single site that checks a condition's type, so
  closing it means adding one.
- **A `catch` binding still has no callable members** — `$e->getMessage()` panics
  `nvs-ir` at `lower/expr.rs:2281`. Unchanged, and it is why both refusal cases
  print a marker from the `catch` arm rather than the message.
- **`orient.py`'s pack was complete for this item.** The two standing manifest
  gaps are unchanged — `[context] modules` has no `nvs-runtime` and no
  `nvs-diagnostics` entry, and `crates/nvs-diagnostics/src/lib.rs` was again a
  file this session edited.

## Next group

**The roster-agreement case and the condition hole, over the two backend files
this session did not open plus the lowering one.** The files:
`crates/nvs-codegen/src/emit.rs`, `crates/nvs-ir/src/lower/expr.rs` and
`crates/nvs-ir/src/lower/convert.rs`.

- [ ] **A `.nvst` case for the five roster comments' testable claims** — the
      catch-all rosters in `crates/nvs-codegen/src/emit.rs` (`emit_binop`,
      `emit_unop`) and `crates/nvs-ir/src/lower/expr.rs:@lower_expr` each claim a
      closed list; the *Agreement* shape in `docs/agent/conventions.md` is the one
      that asserts they agree rather than what each answered.
- [ ] **A `void` call in a condition** — `crates/nvs-ir/src/lower/convert.rs:567`
      panics with *"got Void"* for `if (V::nothing())`, `while`, `?:`, `&&`, `||`
      and `!` alike. ADR 0035 § 1 is the section; the refusal belongs beside the
      operator one, reusing `code::E_VOID_IS_NOT_AN_OPERAND`
      (`crates/nvs-types/src/expr/operators.rs:@report_void_operand`) or taking
      `E0719` if the wording has to differ, and `nvs-types` needs the condition
      site it does not yet have — `crates/nvs-types/src/expr/mod.rs:318` is where
      the truthy path is joined today.
- [ ] **The two panics the first item's roster turns up**, if it turns any up —
      a roster that cannot be asserted is a claim, and the case is what finds out
      which of the five are which.

## Backlog

- A `catch` binding's members — `$e->getMessage()` panics `nvs-ir` at
  `lower/expr.rs:2281`, for `Throwable` and for a named class alike.
- `array<T> as array<U>` where `U` is a class, an enum, a literal or a union is
  `E0711`; `docs/agent/loop-goal.md` § *Standing decisions* owns why.
- `[context] modules` in `docs/agent/loop-goal.toml` has no `nvs-runtime` and no
  `nvs-diagnostics` pattern, and sessions keep editing both.
- `python tools/holes.py` is the worklist for what `nvs-ir`/`nvs-codegen` still
  refuse below the front end; `--item N` prints one in full.
