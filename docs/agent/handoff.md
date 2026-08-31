# Handoff

## State

**ADR 0125 § 2's `as ?class<T>` is landed end to end**, and it was an ICE before this session
rather than a refusal — the checker accepted it and `convert_or_null` panicked. `?class<T>` now
erases to `Ty::ClassDesc` rather than `Ty::Tagged`: no class lives at address zero, so the
representation already had a spare value for `null` and `InstKind::ClassDescIn`'s miss *is* the
answer. `crates/nvs-ir/src/ty.rs`'s `Ty::ClassDesc` doc owns that decision and names the six
lowering sites it obliges; the playbook bullet owns the trap.

**A written-out `::class` operand is now decided at compile time under both spellings.**
`Rock::class as ?class<Animal>` is `E0708`, the same refusal the checked form gives, because § 2
decides it where it stands and there is no run-time throw for the `?` to answer `null` for.

**The driver's acceptance failure is Stage 8's standing differential shortfall, not a
regression.** The check wants 210 passing differential cases and the suite holds 206; the plan's
*Open now* has tracked that count for several sessions. It is the next group, below, because it
is what the check needs — `python tools/gaps.py` names only two members with a PHP twin and no
oracle case, so the other two cases are new depth over members that already have one.

**Known gaps, narrowed.** `$cls::f(...)` written as ADR 0027's first-class callable still records
`ExprInfo::CallableRef`, so the `Closure` names `T`'s method rather than the implementor's. A
dynamic `instanceof` still narrows nothing (`crates/nvs-types/src/locals.rs:481` narrows off
`ExprInfo::InstanceOf { class }`, which the dynamic site deliberately does not record) and
nothing in the reference claims either way.

**`orient.py`'s `[context]` gaps.** Standing and unfixed: no field selects
`docs/reference/lang/*.md`, and this item edited one. `adrs` did not name ADR 0125 § 2 even
though the item cites it by number, nor ADR 0066 § 3, which is the other half of this rule.
`modules` does not name `crates/nvs-ir/src/lower/operator.rs` or `crates/nvs-ir/src/ty.rs`, both
of which this item had to edit. `docs/adr/README.md` and `ground-rules.md` are not in `modules`,
the pack prints the goal item but not the `[[check]]` grading it, and `modules` still names
`crates/nvs-host/src/budget.rs`, which matches nothing.

## Next group

**Four differential cases, to close the acceptance check the driver has failed twice. File set
`tests/differential/core/` and the two `Core` modules whose rows the first two cover.** An oracle
case needs no frozen output — PHP computes the expectation — and PHP is on `PATH` on both legs.
Never under `tests/conformance/` (conventions.md).

- [ ] **`Core\Math::fdiv` gets its oracle case.** `gaps.py` names it as a member with a PHP twin
      (`fdiv`) and no oracle case at all. The row is at `crates/nvs-stdlib/src/math.rs:162` and
      the body at `crates/nvs-stdlib/src/math.rs:1822`; the case is the division-by-zero and
      `NaN` rows PHP's own `fdiv` answers, which is where the two could disagree.
- [ ] **`Core\Task::afterResponse` gets its oracle case.** The other member `gaps.py` names, twin
      `fastcgi_finish_request`, at `crates/nvs-stdlib/src/task.rs:561`. Check first that the CLI
      leg can run it at all — if the twin has no meaning outside a request, say so in the case's
      `--TEST--` and cover the observable half.
- [ ] **`Core\Math::lcm` and `Core\Math::hypot` get oracle cases**, rows at
      `crates/nvs-stdlib/src/math.rs:180` and `crates/nvs-stdlib/src/math.rs:207`. Both sit at
      `gaps.py`'s depth floor of 3 and both have exact PHP twins, so a bound asserted on both
      sides (the last `lcm` that fits an `int` and the first that does not) is the shape.

## Backlog

- **Decide what `$x instanceof $cls` narrows** — `crates/nvs-types/src/locals.rs:481` reads
  `ExprInfo::InstanceOf { class }`, which the dynamic site does not record; ADR 0125 § 4.
- **`$cls::f(...)` as a first-class callable** — `crates/nvs-types/src/expr_table.rs:412`'s
  `ExprInfo::CallableRef` names `T`'s method rather than the implementor's; ADR 0027.
- **`as class<T>`'s throw names the bound, not the name that failed** — § 2 asks for the
  offending class; `crates/nvs-ir/src/lower/convert.rs`'s `lower_class_reference` *Known gaps*.
- **`?class<T>` widened to `mixed` is indistinguishable from `null`** — both are a `Tag::Null`
  byte; `crates/nvs-ir/src/ty.rs`'s `Ty::ClassDesc` says why a descriptor has no tag of its own.
- **Stage 8's other two counts** — conformance 1091 and migration 37% over its 36% floor;
  `docs/implementation-plan.md` *Open now*.
